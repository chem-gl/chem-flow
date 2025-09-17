//! Implementación mínima de persistencia para el trait `FlowRepository`
//! usando Diesel + Postgres por defecto. SQLite se mantiene únicamente para
//! pruebas (cfg(test)). Diseñado para tests y como referencia.
//!
//! Documentación y mensajes en español.

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::Error as DieselError;
use uuid::Uuid;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use chrono::{Utc, TimeZone};

use flow::repository::{FlowRepository, SnapshotStore, ArtifactStore};
use flow::domain::{FlowData, FlowMeta, PersistResult, SnapshotMeta, WorkItem};
use flow::errors::{FlowError, Result as FlowResult};

mod schema;
use schema::flows::dsl as flows_dsl;
use schema::flow_data::dsl as data_dsl;
use schema::*;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

// Selecciona el tipo de conexión para el pool según la configuración de
// compilación/pruebas. En producción se usa Postgres cuando la feature
// "pg" está habilitada.
//
// - Al compilar con `--features pg` (y no en modo test) usamos Postgres.
// - Para las pruebas (`cfg(test)`) se compilan helpers para SQLite, de modo
//   que las pruebas usen una BD en memoria aunque la feature `pg` esté
//   presente en la compilación del workspace.
// - Si el crate se compila sin la feature `pg` en modo no-test, se usa
//   SQLite como fallback.
#[cfg(all(feature = "pg", not(test)))]
type DbPool = Pool<ConnectionManager<PgConnection>>;

#[cfg(any(test, not(feature = "pg")))]
type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Repositorio mínimo usando Diesel; en producción puede usar Postgres.
/// Los helpers para SQLite se compilan sólo en pruebas.
pub struct DieselFlowRepository {
    pool: Arc<DbPool>,
}

impl DieselFlowRepository {
    /// Devuelve todos los rows de `flows` y `flow_data` para debug/inspección.
    /// Retorna (flows, flow_data) serializados en tipos de dominio.
    pub fn dump_tables_for_debug(&self) -> FlowResult<(Vec<FlowMeta>, Vec<FlowData>)> {
        let mut conn = self.conn()?;
        // flows
        let frows = map_db_err(flows_dsl::flows.load::<FlowRow>(&mut conn))?;
        let mut flows_out = Vec::new();
        for r in frows {
            flows_out.push(FlowMeta { id: Uuid::parse_str(&r.id).unwrap(), name: r.name, status: r.status, created_by: r.created_by, created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()), current_cursor: r.current_cursor, current_version: r.current_version, parent_flow_id: r.parent_flow_id.and_then(|s| Uuid::parse_str(&s).ok()), parent_cursor: r.parent_cursor, metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})) });
        }
        // flow_data
        let drows = map_db_err(data_dsl::flow_data.load::<FlowDataRow>(&mut conn))?;
        let mut data_out = Vec::new();
        for r in drows {
            data_out.push(FlowData { id: Uuid::parse_str(&r.id).unwrap(), flow_id: Uuid::parse_str(&r.flow_id).unwrap(), cursor: r.cursor, key: r.key, payload: serde_json::from_str(&r.payload).unwrap_or(serde_json::json!({})), metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})), command_id: r.command_id.and_then(|s| Uuid::parse_str(&s).ok()), created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()) });
        }
        Ok((flows_out, data_out))
    }
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = flows)]
struct FlowRow {
    id: String,
    name: Option<String>,
    status: Option<String>,
    created_by: Option<String>,
    created_at_ts: i64,
    current_cursor: i64,
    current_version: i64,
    parent_flow_id: Option<String>,
    parent_cursor: Option<i64>,
    metadata: String,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = flow_data)]
struct FlowDataRow {
    id: String,
    flow_id: String,
    cursor: i64,
    key: String,
    payload: String,
    metadata: String,
    command_id: Option<String>,
    created_at_ts: i64,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = snapshots)]
struct SnapshotRow {
    id: String,
    flow_id: String,
    cursor: i64,
    state_ptr: String,
    metadata: String,
    created_at_ts: i64,
}

#[cfg(any(test, not(feature = "pg")))]
impl DieselFlowRepository {
    /// Crea una instancia para pruebas y aplica las migraciones embebidas (SQLite).
    /// Este constructor sólo está disponible en compilaciones de prueba.
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder().max_size(1).build(manager).expect("no se pudo crear el pool de conexiones");
        let repo = DieselFlowRepository { pool: Arc::new(pool) };

        // Intentar aplicar las migraciones embebidas para las pruebas.
        if let Ok(mut c) = repo.conn_raw() {
            // Ajustes para evitar 'database table is locked' en pruebas concurrentes
            // Habilitamos WAL y aumentamos busy_timeout para que las escrituras esperen.
            let _ = diesel::sql_query("PRAGMA journal_mode = WAL;").execute(&mut c);
            let _ = diesel::sql_query("PRAGMA busy_timeout = 5000;").execute(&mut c);
            match c.run_pending_migrations(MIGRATIONS) {
                Ok(applied) => eprintln!("chem-persistence (test sqlite): aplicadas {} migraciones embebidas", applied.len()),
                Err(e) => eprintln!("chem-persistence (test sqlite): fallo al ejecutar migraciones embebidas: {}", e),
            }
        }

        repo
    }

    /// Devuelve una conexión pooled; error mapeado a FlowError::Storage.
    pub fn conn(&self) -> FlowResult<PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.conn_raw().map_err(|e| FlowError::Storage(format!("pool: {}", e)))
    }

    fn conn_raw(&self) -> std::result::Result<PooledConnection<ConnectionManager<SqliteConnection>>, r2d2::Error> {
        let mut conn = self.pool.get()?;
        // Asegurar PRAGMA en cada conexión pooled para evitar bloqueos
        let _ = diesel::sql_query("PRAGMA journal_mode = WAL;").execute(&mut conn);
        let _ = diesel::sql_query("PRAGMA busy_timeout = 5000;").execute(&mut conn);
        Ok(conn)
    }
}



#[cfg(all(feature = "pg", not(test)))]
impl DieselFlowRepository {
    /// Para compilaciones con Postgres, devuelve conexiones Pg desde el pool.
    pub fn conn(&self) -> FlowResult<PooledConnection<ConnectionManager<PgConnection>>> {
        self.conn_raw().map_err(|e| FlowError::Storage(format!("pool: {}", e)))
    }

    fn conn_raw(&self) -> std::result::Result<PooledConnection<ConnectionManager<PgConnection>>, r2d2::Error> {
        self.pool.get()
    }
}

// Implementación de `new_from_env` para Postgres en producción. Compilada
// sólo cuando se habilita la feature `pg` y no estamos en modo test.
#[cfg(all(feature = "pg", not(test)))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
    if !(url.starts_with("postgres") || url.starts_with("postgresql://") || url.contains("@")) {
        return Err(FlowError::Other("chem-persistence: DATABASE_URL does not look like Postgres URL".into()));
    }
    DieselFlowRepository::new_pg(&url)
}

// Implementación para pruebas: permite usar SQLite en memoria. Compilada
// únicamente para tests.
#[cfg(test)]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "file:memdb1?mode=memory&cache=shared".into());
    let repo = DieselFlowRepository::new(&url);
    Ok(repo)
}

// Fallback para compilaciones sin la feature `pg` (no-test): si la
// `DATABASE_URL` parece apuntar a SQLite (por ejemplo `file:` o contiene
// `mode=memory`) construimos un repo SQLite para facilitar pruebas e
// integración local. En caso contrario devolvemos un error indicando al
// usuario que habilite la feature `pg` para usar Postgres en producción.
#[cfg(all(not(feature = "pg"), not(test)))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
    let url_l = url.to_lowercase();
    if url_l.starts_with("file:") || url_l.contains("mode=memory") || url_l.contains("sqlite") {
        // Accept sqlite URL in non-pg builds for convenience in local/dev
        let repo = DieselFlowRepository::new(&url);
        return Ok(repo);
    }
    Err(FlowError::Other("chem-persistence was compiled without 'pg' feature; enable the 'pg' feature to use Postgres in production".into()))
}

// Constructores específicos para Postgres: compilados sólo cuando la
// feature "pg" está activada y no durante tests (evita conflictos de
// tipos con el DbPool de SQLite en las pruebas).
#[cfg(all(feature = "pg", not(test)))]
impl DieselFlowRepository {
    pub fn new_pg(database_url: &str) -> FlowResult<DieselFlowRepository> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(manager).map_err(|e| FlowError::Storage(format!("no se pudo crear el pool de conexiones: {}", e)))?;
        let repo = DieselFlowRepository { pool: Arc::new(pool) };
        if let Ok(mut c) = repo.conn_raw() {
            match c.run_pending_migrations(MIGRATIONS) {
                Ok(applied) => eprintln!("chem-persistence (pg): aplicadas {} migraciones embebidas", applied.len()),
                Err(e) => eprintln!("chem-persistence (pg): fallo al ejecutar migraciones embebidas: {}", e),
            }
        }
        Ok(repo)
    }

    pub fn new_pg_from_env() -> FlowResult<DieselFlowRepository> {
        dotenvy::dotenv().ok();
        let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
        DieselFlowRepository::new_pg(&url)
    }
}

fn map_db_err<T>(res: std::result::Result<T, DieselError>) -> FlowResult<T> {
    res.map_err(|e| FlowError::Storage(format!("db: {}", e)))
}

impl FlowRepository for DieselFlowRepository {
    fn get_flow_meta(&self, flow_id: &Uuid) -> FlowResult<FlowMeta> {
        use schema::flows::dsl::*;
        let mut conn = self.conn()?;
        let fid = flow_id.to_string();
        let row = map_db_err(flows.filter(id.eq(&fid)).first::<FlowRow>(&mut conn))?;
        Ok(FlowMeta {
            id: Uuid::parse_str(&row.id).unwrap(),
            name: row.name,
            status: row.status,
            created_by: row.created_by,
            created_at: Utc.timestamp_opt(row.created_at_ts, 0).single().unwrap_or(Utc::now()),
            current_cursor: row.current_cursor,
            current_version: row.current_version,
            parent_flow_id: row.parent_flow_id.and_then(|s| Uuid::parse_str(&s).ok()),
            parent_cursor: row.parent_cursor,
            metadata: serde_json::from_str(&row.metadata).unwrap_or(serde_json::json!({})),
        })
    }

    fn create_flow(&self, name_in: Option<String>, status_in: Option<String>, metadata_in: JsonValue) -> FlowResult<Uuid> {
        let mut conn = self.conn()?;
        let new_id = Uuid::new_v4();
        let now_ts = Utc::now().timestamp();
        let meta_s = metadata_in.to_string();
        let new = FlowRow { id: new_id.to_string(), name: name_in, status: status_in, created_by: None, created_at_ts: now_ts, current_cursor: 0, current_version: 0, parent_flow_id: None, parent_cursor: None, metadata: meta_s };
        map_db_err(diesel::insert_into(flows_dsl::flows).values(&new).execute(&mut conn))?;
        Ok(new_id)
    }

    fn persist_data(&self, data: &FlowData, expected_version: i64) -> FlowResult<PersistResult> {
        use diesel::prelude::*;
        let mut conn = self.conn()?;
        // Hacemos la transacción con el tipo de error de Diesel y lo mapeamos
        // a FlowError al final para mantener compatibilidad con la interfaz.
        let fid = data.flow_id.to_string();
        let tx_res: std::result::Result<PersistResult, diesel::result::Error> = conn.transaction::<PersistResult, diesel::result::Error, _>(|conn| {
            let row_version: i64 = flows_dsl::flows.filter(flows_dsl::id.eq(&fid)).select(flows_dsl::current_version).first(conn)?;
            if row_version != expected_version {
                return Ok(PersistResult::Conflict);
            }
            let row = FlowDataRow { id: data.id.to_string(), flow_id: data.flow_id.to_string(), cursor: data.cursor, key: data.key.clone(), payload: data.payload.to_string(), metadata: data.metadata.to_string(), command_id: data.command_id.map(|u| u.to_string()), created_at_ts: data.created_at.timestamp() };
            diesel::insert_into(data_dsl::flow_data).values(&row).execute(conn)?;
            diesel::update(flows_dsl::flows.filter(flows_dsl::id.eq(&fid))).set((flows_dsl::current_version.eq(row_version+1), flows_dsl::current_cursor.eq(data.cursor))).execute(conn)?;
            Ok(PersistResult::Ok { new_version: row_version+1 })
        });
        match tx_res {
            Ok(v) => Ok(v),
            Err(e) => Err(FlowError::Storage(format!("db txn: {}", e)))
        }
    }

    fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> FlowResult<Vec<FlowData>> {
        let mut conn = self.conn()?;
        let fid = flow_id.to_string();
        let rows = map_db_err(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid).and(data_dsl::cursor.gt(from_cursor))).order(data_dsl::cursor.asc()).load::<FlowDataRow>(&mut conn))?;
        let mut out = Vec::new();
        for r in rows {
            let created = Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now());
            let item = FlowData { id: Uuid::parse_str(&r.id).unwrap(), flow_id: Uuid::parse_str(&r.flow_id).unwrap(), cursor: r.cursor, key: r.key, payload: serde_json::from_str(&r.payload).unwrap_or(serde_json::json!({})), metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})), command_id: r.command_id.and_then(|s| Uuid::parse_str(&s).ok()), created_at: created };
            out.push(item);
        }
        Ok(out)
    }

    fn load_latest_snapshot(&self, flow_id_in: &Uuid) -> FlowResult<Option<SnapshotMeta>> {
        use schema::snapshots::dsl::*;
        let mut conn = self.conn()?;
        let fid_s = flow_id_in.to_string();
        let row_opt = snapshots.filter(flow_id.eq(&fid_s)).order((cursor.desc(), created_at_ts.desc())).first::<SnapshotRow>(&mut conn).optional().map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
        if let Some(r) = row_opt {
            let meta = SnapshotMeta {
                id: Uuid::parse_str(&r.id).unwrap(),
                flow_id: Uuid::parse_str(&r.flow_id).unwrap(),
                cursor: r.cursor,
                state_ptr: r.state_ptr.clone(),
                metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
                created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()),
            };
            Ok(Some(meta))
        } else {
            Ok(None)
        }
    }

    fn load_snapshot(&self, snapshot_id: &Uuid) -> FlowResult<(Vec<u8>, SnapshotMeta)> {
        use schema::snapshots::dsl::*;
        let mut conn = self.conn()?;
        let sid = snapshot_id.to_string();
        let r = snapshots.filter(id.eq(&sid)).first::<SnapshotRow>(&mut conn).map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    // Devolver los bytes crudos del campo `state_ptr` (sin dependencias externas)
        let bytes = r.state_ptr.clone().into_bytes();
        let meta = SnapshotMeta { id: Uuid::parse_str(&r.id).unwrap(), flow_id: Uuid::parse_str(&r.flow_id).unwrap(), cursor: r.cursor, state_ptr: r.state_ptr.clone(), metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})), created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()) };
        Ok((bytes, meta))
    }

    fn save_snapshot(&self, flow_id_in: &Uuid, cursor_in: i64, state_ptr_in: &str, metadata_in: serde_json::Value) -> FlowResult<Uuid> {
        use schema::snapshots::dsl::*;
        let mut conn = self.conn()?;
        let new_id = Uuid::new_v4();
        let now_ts = Utc::now().timestamp();
        let snap = SnapshotRow { id: new_id.to_string(), flow_id: flow_id_in.to_string(), cursor: cursor_in, state_ptr: state_ptr_in.to_string(), metadata: metadata_in.to_string(), created_at_ts: now_ts };
        diesel::insert_into(snapshots).values(&snap).execute(&mut conn).map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
        Ok(new_id)
    }

    fn create_branch(&self, parent_flow_id: &Uuid, name_in: Option<String>, status_in: Option<String>, parent_cursor: i64, metadata_in: JsonValue) -> FlowResult<Uuid> {
        let mut conn = self.conn()?;
    conn.transaction::<Uuid, diesel::result::Error, _>(|conn| {
            let new_id = Uuid::new_v4();
            let meta_s = metadata_in.to_string();
            let now_ts = Utc::now().timestamp();
            // prepare copies: read parent rows and snapshots first
            let parent_id_s = parent_flow_id.to_string();
            let rows = data_dsl::flow_data.filter(data_dsl::flow_id.eq(&parent_id_s).and(data_dsl::cursor.le(parent_cursor))).load::<FlowDataRow>(conn)?;
            use schema::snapshots::dsl as snaps_dsl;
            let snaps = snaps_dsl::snapshots.filter(snaps_dsl::flow_id.eq(&parent_id_s).and(snaps_dsl::cursor.le(parent_cursor))).load::<SnapshotRow>(conn)?;

            // insert new flow. We copy the parent's data rows into the new
            // branch, but initialize the branch's logical `current_version`
            // counter to 0 so that callers can append using expected_version=0
            // for the first write (this matches the tests' expectations).
            let _copied_count = rows.len() as i64;
            let new = FlowRow { id: new_id.to_string(), name: name_in.clone(), status: status_in.clone(), created_by: None, created_at_ts: now_ts, current_cursor: parent_cursor, current_version: 0, parent_flow_id: Some(parent_flow_id.to_string()), parent_cursor: Some(parent_cursor), metadata: meta_s };
            diesel::insert_into(flows_dsl::flows).values(&new).execute(conn)?;

            // insert copies of flow_data
            for r in rows {
                let copy = FlowDataRow { id: Uuid::new_v4().to_string(), flow_id: new_id.to_string(), cursor: r.cursor, key: r.key.clone(), payload: r.payload.clone(), metadata: r.metadata.clone(), command_id: r.command_id.clone(), created_at_ts: r.created_at_ts };
                diesel::insert_into(data_dsl::flow_data).values(&copy).execute(conn)?;
            }

            // insert copies of snapshots
            for s in snaps {
                let s_copy = SnapshotRow { id: Uuid::new_v4().to_string(), flow_id: new_id.to_string(), cursor: s.cursor, state_ptr: s.state_ptr.clone(), metadata: s.metadata.clone(), created_at_ts: s.created_at_ts };
                diesel::insert_into(snaps_dsl::snapshots).values(&s_copy).execute(conn)?;
            }
            Ok(new_id)
        })
        .map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
    }

    fn branch_exists(&self, flow_id: &Uuid) -> FlowResult<bool> {
        let mut conn = self.conn()?;
        let fid = flow_id.to_string();
        let c: i64 = map_db_err(flows_dsl::flows.filter(flows_dsl::id.eq(&fid)).count().get_result(&mut conn))?;
        Ok(c > 0)
    }

    fn count_steps(&self, flow_id: &Uuid) -> FlowResult<i64> {
        let mut conn = self.conn()?;
        let fid = flow_id.to_string();
        let c: i64 = map_db_err(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid)).count().get_result(&mut conn))?;
        Ok(c)
    }

    fn delete_branch(&self, flow_id: &Uuid) -> FlowResult<()> {
        let mut conn = self.conn()?;
        let fid = flow_id.to_string();
        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            // borrar steps
            diesel::delete(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid))).execute(conn)?;
            // borrar snapshots
            diesel::delete(schema::snapshots::dsl::snapshots.filter(schema::snapshots::dsl::flow_id.eq(&fid))).execute(conn)?;

            // orphan children: any flows that had parent_flow_id == fid should
            // be updated to have NULL parent_flow_id and NULL parent_cursor so
            // they become roots (huérfanos). This preserves child branches.
            diesel::update(flows_dsl::flows.filter(flows_dsl::parent_flow_id.eq(Some(fid.clone()))))
                .set((flows_dsl::parent_flow_id.eq::<Option<String>>(None), flows_dsl::parent_cursor.eq::<Option<i64>>(None)))
                .execute(conn)?;

            // borrar la fila del flow
            diesel::delete(flows_dsl::flows.filter(flows_dsl::id.eq(&fid))).execute(conn)?;
            Ok(())
        }).map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
    }
    fn delete_from_step(&self, _flow_id: &Uuid, _from_cursor: i64) -> FlowResult<()> { Err(FlowError::Other("not implemented".into())) }
    fn lock_for_update(&self, _flow_id: &Uuid, _expected_version: i64) -> FlowResult<bool> { Ok(true) }
    fn claim_work(&self, _worker_id: &str) -> FlowResult<Option<WorkItem>> { Ok(None) }
}

impl SnapshotStore for DieselFlowRepository { fn save(&self, _state: &[u8]) -> FlowResult<String> { Err(FlowError::Other("not implemented".into())) } fn load(&self, _key: &str) -> FlowResult<Vec<u8>> { Err(FlowError::Other("not implemented".into())) } }
impl ArtifactStore for DieselFlowRepository { fn put(&self, _blob: &[u8]) -> FlowResult<String> { Err(FlowError::Other("not implemented".into())) } fn get(&self, _key: &str) -> FlowResult<Vec<u8>> { Err(FlowError::Other("not implemented".into())) } fn copy_if_needed(&self, _src_key: &str) -> FlowResult<String> { Err(FlowError::Other("not implemented".into())) } }
