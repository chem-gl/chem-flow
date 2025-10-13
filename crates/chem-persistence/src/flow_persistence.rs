//! Lógica de persistencia para el dominio (migrado desde lib.rs)
use crate::db::run_migrations_on_pool;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::Error as DieselError;
use flow::domain::{FlowData, FlowMeta, FlowMetadata, PersistResult, SnapshotMeta, WorkItem};
use flow::errors::{FlowError, Result as FlowResult};
use flow::ports::outbound::{
  BranchInfo, BranchManagementPort, FlowDataPort, FlowMetadataPort, FlowRepository as OutboundFlowRepository, MetadataPort,
  RepositoryStats, SnapshotPort,
};
use flow::repository::{ArtifactStore, FlowRepository, SnapshotStore};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
// Reusar el módulo `schema` definido en `lib.rs`.
use crate::schema;
use crate::schema::flow_data::dsl as data_dsl;
use crate::schema::flows::dsl as flows_dsl;
use crate::schema::*;
#[cfg(feature = "postgres")]
type DbPool = Pool<ConnectionManager<PgConnection>>;
#[cfg(not(feature = "postgres"))]
type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub struct DieselFlowRepository {
  pool: Arc<DbPool>,
  snapshot_dir: String,
  artifact_dir: String,
}
#[cfg(all(test, feature = "sqlite", not(feature = "postgres")))]
struct FlowTestGuard {
  _db: crate::test_helpers::TestSqliteDb,
  snapshot_dir: std::path::PathBuf,
  artifact_dir: std::path::PathBuf,
}
#[cfg(all(test, feature = "sqlite", not(feature = "postgres")))]
impl Drop for FlowTestGuard {
  fn drop(&mut self) {
    let _ = std::fs::remove_dir_all(&self.snapshot_dir);
    let _ = std::fs::remove_dir_all(&self.artifact_dir);
  }
}
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = flows)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
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
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = flow_data)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
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
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct SnapshotRow {
  id: String,
  flow_id: String,
  cursor: i64,
  state_ptr: String,
  metadata: String,
  created_at_ts: i64,
}
impl DieselFlowRepository {
  fn default_snapshot_dir() -> String {
    std::env::var("SNAPSHOT_DIR").unwrap_or_else(|_| "./snapshots".to_string())
  }
  fn default_artifact_dir() -> String {
    std::env::var("ARTIFACT_DIR").unwrap_or_else(|_| "./artifacts".to_string())
  }
  pub fn new_with_pool(pool: DbPool) -> FlowResult<Self> {
    Self::with_pool_and_dirs(pool, Self::default_snapshot_dir(), Self::default_artifact_dir())
  }
  pub fn with_pool_and_dirs(pool: DbPool, snapshot_dir: String, artifact_dir: String) -> FlowResult<Self> {
    fs::create_dir_all(&snapshot_dir).map_err(|e| FlowError::Storage(format!("snapshot dir: {}", e)))?;
    fs::create_dir_all(&artifact_dir).map_err(|e| FlowError::Storage(format!("artifact dir: {}", e)))?;
    run_migrations_on_pool(&pool).map_err(|e| FlowError::Storage(format!("migrations: {}", e)))?;
    Ok(DieselFlowRepository { pool: Arc::new(pool), snapshot_dir, artifact_dir })
  }
  pub fn pool(&self) -> Arc<DbPool> {
    Arc::clone(&self.pool)
  }
}
#[cfg(not(feature = "postgres"))]
impl DieselFlowRepository {
  pub fn try_new(database_url: &str) -> FlowResult<Self> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder().max_size(1).build(manager).expect("no se pudo crear el pool de conexiones");
    DieselFlowRepository::new_with_pool(pool)
  }
  pub fn new(database_url: &str) -> Self {
    DieselFlowRepository::try_new(database_url).expect("failed to initialize DieselFlowRepository")
  }
  pub fn conn(&self) -> FlowResult<PooledConnection<ConnectionManager<SqliteConnection>>> {
    self.conn_raw().map_err(|e| FlowError::Storage(format!("pool: {}", e)))
  }
  fn conn_raw(&self) -> std::result::Result<PooledConnection<ConnectionManager<SqliteConnection>>, r2d2::Error> {
    #[cfg_attr(feature = "postgres", allow(unused_mut))]
    let mut conn = self.pool.get()?;
    let _ = diesel::sql_query("PRAGMA journal_mode = WAL;").execute(&mut conn);
    let _ = diesel::sql_query("PRAGMA busy_timeout = 5000;").execute(&mut conn);
    Ok(conn)
  }
}
#[cfg(feature = "postgres")]
impl DieselFlowRepository {
  pub fn conn(&self) -> FlowResult<PooledConnection<ConnectionManager<PgConnection>>> {
    self.conn_raw().map_err(|e| FlowError::Storage(format!("pool: {}", e)))
  }
  fn conn_raw(&self) -> std::result::Result<PooledConnection<ConnectionManager<PgConnection>>, r2d2::Error> {
    self.pool.get()
  }
}
#[cfg(all(feature = "postgres", not(test)))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
  dotenvy::dotenv().ok();
  let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
  if !(url.starts_with("postgres") || url.starts_with("postgresql://") || url.contains("@")) {
    return Err(FlowError::Other("chem-persistence: DATABASE_URL does not look like Postgres URL".into()));
  }
  DieselFlowRepository::new_pg(&url)
}
#[cfg(all(test, feature = "sqlite", not(feature = "postgres")))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
  use crate::test_helpers::create_temp_sqlite_db;
  use once_cell::sync::Lazy;
  use std::sync::Mutex;
  static FLOW_GUARDS: Lazy<Mutex<Vec<FlowTestGuard>>> = Lazy::new(|| Mutex::new(Vec::new()));
  let db = create_temp_sqlite_db().map_err(|e| FlowError::Other(format!("sqlite helper: {}", e)))?;
  let snapshot_dir = std::env::temp_dir().join(format!("chemflow_snapshots_{}", Uuid::new_v4()));
  let artifact_dir = std::env::temp_dir().join(format!("chemflow_artifacts_{}", Uuid::new_v4()));
  let repo = DieselFlowRepository::with_pool_and_dirs(db.pool.clone(),
                                                      snapshot_dir.to_string_lossy().into_owned(),
                                                      artifact_dir.to_string_lossy().into_owned())?;
  let guard = FlowTestGuard { _db: db, snapshot_dir, artifact_dir };
  FLOW_GUARDS.lock().expect("poisoned sqlite flow guard mutex").push(guard);
  Ok(repo)
}
#[cfg(all(test, feature = "postgres"))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
  dotenvy::dotenv().ok();
  let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
  DieselFlowRepository::new_pg(&url)
}
#[cfg(all(not(feature = "postgres"), not(test)))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
  dotenvy::dotenv().ok();
  let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
  let url_l = url.to_lowercase();
  if url_l.contains("mode=memory") {
    return Err(FlowError::Other("chem-persistence: in-memory sqlite URLs are not supported; provide a file-backed path"
                                               .into()));
  }
  if url_l.starts_with("file:") || url_l.contains("sqlite") {
    let repo = DieselFlowRepository::new(&url);
    return Ok(repo);
  }
  Err(FlowError::Other("chem-persistence was compiled without 'pg' feature; enable the 'pg' feature to use Postgres in \
                        production"
                                   .into()))
}
#[cfg(all(test, not(feature = "sqlite"), not(feature = "postgres")))]
pub fn new_from_env() -> FlowResult<DieselFlowRepository> {
  dotenvy::dotenv().ok();
  let url = std::env::var("DATABASE_URL").map_err(|_| FlowError::Other("DATABASE_URL not set".into()))?;
  let url_l = url.to_lowercase();
  if url_l.contains("mode=memory") {
    return Err(FlowError::Other("chem-persistence: in-memory sqlite URLs are not supported; provide a file-backed path".into()));
  }
  if url_l.starts_with("file:") || url_l.contains("sqlite") {
    return DieselFlowRepository::try_new(&url);
  }
  Err(FlowError::Other("chem-persistence requires either the 'postgres' or 'sqlite' feature".into()))
}
#[cfg(feature = "postgres")]
impl DieselFlowRepository {
  pub fn new_pg(database_url: &str) -> FlowResult<DieselFlowRepository> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(manager)
                              .map_err(|e| FlowError::Storage(format!("no se pudo crear el pool de conexiones: {}", e)))?;
    DieselFlowRepository::new_with_pool(pool)
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
#[async_trait]
impl FlowMetadataPort for DieselFlowRepository {
  async fn get_flow_metadata(&self, flow_id: &Uuid) -> FlowResult<FlowMetadata> {
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let row = map_db_err(flows.select(FlowRow::as_select()).filter(id.eq(&fid)).first::<FlowRow>(&mut conn))?;
    Ok(FlowMetadata { id: Uuid::parse_str(&row.id).unwrap(),
                      name: row.name,
                      status: row.status,
                      created_by: row.created_by,
                      created_at: Utc.timestamp_opt(row.created_at_ts, 0).single().unwrap_or(Utc::now()),
                      current_cursor: row.current_cursor,
                      current_version: row.current_version,
                      parent_flow_id: row.parent_flow_id.and_then(|s| Uuid::parse_str(&s).ok()),
                      parent_cursor: row.parent_cursor,
                      metadata: serde_json::from_str(&row.metadata).unwrap_or(serde_json::json!({})) })
  }

  async fn create_flow(&self, metadata: FlowMetadata) -> FlowResult<Uuid> {
    let mut conn = self.conn()?;
    let new_id = metadata.id;
    let now_ts = metadata.created_at.timestamp();
    let meta_s = metadata.metadata.to_string();
    let new = FlowRow { id: new_id.to_string(),
                        name: metadata.name,
                        status: metadata.status,
                        created_by: metadata.created_by,
                        created_at_ts: now_ts,
                        current_cursor: metadata.current_cursor,
                        current_version: metadata.current_version,
                        parent_flow_id: metadata.parent_flow_id.map(|u| u.to_string()),
                        parent_cursor: metadata.parent_cursor,
                        metadata: meta_s };
    map_db_err(diesel::insert_into(flows_dsl::flows).values(&new).execute(&mut conn))?;
    Ok(new_id)
  }

  async fn update_flow_metadata(&self, flow_id: &Uuid, flow_metadata: FlowMetadata) -> FlowResult<()> {
    use diesel::prelude::*;
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let meta_s = flow_metadata.metadata.to_string();
    map_db_err(diesel::update(flows.filter(id.eq(&fid))).set((name.eq(flow_metadata.name),
                                                              status.eq(flow_metadata.status),
                                                              created_by.eq(flow_metadata.created_by),
                                                              current_cursor.eq(flow_metadata.current_cursor),
                                                              current_version.eq(flow_metadata.current_version),
                                                              parent_flow_id.eq(flow_metadata.parent_flow_id
                                                                                             .map(|u| u.to_string())),
                                                              parent_cursor.eq(flow_metadata.parent_cursor),
                                                              metadata.eq(meta_s)))
                                                        .execute(&mut conn))?;
    Ok(())
  }

  async fn delete_flow(&self, flow_id: &Uuid) -> FlowResult<()> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    // Delete in order: data, snapshots, then flow
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
          diesel::delete(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid))).execute(conn)?;
          diesel::delete(schema::snapshots::dsl::snapshots.filter(schema::snapshots::dsl::flow_id.eq(&fid))).execute(conn)?;
          diesel::delete(flows_dsl::flows.filter(flows_dsl::id.eq(&fid))).execute(conn)?;
          Ok(())
        })
        .map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
  }

  async fn list_flow_ids(&self) -> FlowResult<Vec<Uuid>> {
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let rows = map_db_err(flows.select(id).load::<String>(&mut conn))?;
    let mut out = Vec::new();
    for s in rows {
      if let Ok(u) = Uuid::parse_str(&s) {
        out.push(u);
      }
    }
    Ok(out)
  }

  async fn flow_exists(&self, flow_id: &Uuid) -> FlowResult<bool> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let c: i64 = map_db_err(flows_dsl::flows.filter(flows_dsl::id.eq(&fid)).count().get_result(&mut conn))?;
    Ok(c > 0)
  }

  async fn get_flow_status(&self, flow_id: &Uuid) -> FlowResult<Option<String>> {
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let row_opt = flows.select(status)
                       .filter(id.eq(&fid))
                       .first::<Option<String>>(&mut conn)
                       .optional()
                       .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    Ok(row_opt.flatten())
  }

  async fn set_flow_status(&self, _flow_id: &Uuid, _status: Option<String>) -> FlowResult<()> {
    // TODO: Implement when needed
    Ok(())
  }
}

#[async_trait]
impl FlowDataPort for DieselFlowRepository {
  async fn persist_data(&self, data: &FlowData, expected_version: i64) -> FlowResult<PersistResult> {
    use diesel::prelude::*;
    let mut conn = self.conn()?;
    let fid = data.flow_id.to_string();
    let tx_res: std::result::Result<PersistResult, diesel::result::Error> =
      conn.transaction::<PersistResult, diesel::result::Error, _>(|conn| {
            let row_version: i64 =
              flows_dsl::flows.filter(flows_dsl::id.eq(&fid)).select(flows_dsl::current_version).first(conn)?;
            if row_version != expected_version {
              return Ok(PersistResult::Conflict);
            }
            let row = FlowDataRow { id: data.id.to_string(),
                                    flow_id: data.flow_id.to_string(),
                                    cursor: data.cursor,
                                    key: data.key.clone(),
                                    payload: data.payload.to_string(),
                                    metadata: data.metadata.to_string(),
                                    command_id: data.command_id.map(|u| u.to_string()),
                                    created_at_ts: data.created_at.timestamp() };
            diesel::insert_into(data_dsl::flow_data).values(&row).execute(conn)?;
            diesel::update(flows_dsl::flows.filter(flows_dsl::id.eq(&fid))).set((flows_dsl::current_version.eq(row_version
                                                                                                               + 1),
                                                                                 flows_dsl::current_cursor.eq(data.cursor)))
                                                                           .execute(conn)?;
            Ok(PersistResult::Ok { new_version: row_version + 1 })
          });
    match tx_res {
      Ok(v) => Ok(v),
      Err(e) => Err(FlowError::Storage(format!("db txn: {}", e))),
    }
  }

  async fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> FlowResult<Vec<FlowData>> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let rows = map_db_err(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid).and(data_dsl::cursor.gt(from_cursor)))
                                             .order(data_dsl::cursor.asc())
                                             .load::<FlowDataRow>(&mut conn))?;
    let mut out = Vec::new();
    for r in rows {
      let created = Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now());
      let item = FlowData { id: Uuid::parse_str(&r.id).unwrap(),
                            flow_id: Uuid::parse_str(&r.flow_id).unwrap(),
                            cursor: r.cursor,
                            key: r.key,
                            payload: serde_json::from_str(&r.payload).unwrap_or(serde_json::json!({})),
                            metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
                            command_id: r.command_id.and_then(|s| Uuid::parse_str(&s).ok()),
                            created_at: created };
      out.push(item);
    }
    Ok(out)
  }

  async fn read_data_at_cursor(&self, flow_id: &Uuid, cursor: i64) -> FlowResult<Option<FlowData>> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let row_opt = data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid).and(data_dsl::cursor.eq(cursor)))
                                     .first::<FlowDataRow>(&mut conn)
                                     .optional()
                                     .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(r) = row_opt {
      let created = Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now());
      let item = FlowData { id: Uuid::parse_str(&r.id).unwrap(),
                            flow_id: Uuid::parse_str(&r.flow_id).unwrap(),
                            cursor: r.cursor,
                            key: r.key,
                            payload: serde_json::from_str(&r.payload).unwrap_or(serde_json::json!({})),
                            metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
                            command_id: r.command_id.and_then(|s| Uuid::parse_str(&s).ok()),
                            created_at: created };
      Ok(Some(item))
    } else {
      Ok(None)
    }
  }

  async fn count_flow_data(&self, flow_id: &Uuid) -> FlowResult<i64> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let c: i64 = map_db_err(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid)).count().get_result(&mut conn))?;
    Ok(c)
  }

  async fn delete_data_from_cursor(&self, flow_id: &Uuid, from_cursor: i64) -> FlowResult<()> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
      // Delete flow_data with cursor >= from_cursor
      diesel::delete(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid).and(data_dsl::cursor.ge(from_cursor)))).execute(conn)?;
      // Update current_cursor to max remaining cursor or 0
      let new_cursor_opt: Option<i64> = data_dsl::flow_data
        .filter(data_dsl::flow_id.eq(&fid))
        .select(diesel::dsl::max(data_dsl::cursor))
        .first::<Option<i64>>(conn)?;
      let new_cursor = new_cursor_opt.unwrap_or(0);
      diesel::update(flows_dsl::flows.filter(flows_dsl::id.eq(&fid)))
        .set(flows_dsl::current_cursor.eq(new_cursor))
        .execute(conn)?;
      Ok(())
    }).map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
  }

  async fn content_exists(&self, _content_hash: &str) -> FlowResult<bool> {
    // This is a simplified implementation - in practice you'd need to store hashes
    // For now, just return false to indicate no duplicate checking
    Ok(false)
  }
}

#[async_trait]
impl BranchManagementPort for DieselFlowRepository {
  async fn create_branch(&self, parent_flow_id: &Uuid, parent_cursor: i64, metadata: JsonValue) -> FlowResult<Uuid> {
    let mut conn = self.conn()?;
    conn.transaction::<Uuid, diesel::result::Error, _>(|conn| {
          let new_id = Uuid::new_v4();
          let meta_s = metadata.to_string();
          let now_ts = Utc::now().timestamp();
          let parent_id_s = parent_flow_id.to_string();
          let rows = data_dsl::flow_data.filter(data_dsl::flow_id.eq(&parent_id_s).and(data_dsl::cursor.le(parent_cursor)))
                                        .load::<FlowDataRow>(conn)?;
          use schema::snapshots::dsl as snaps_dsl;
          let snaps = snaps_dsl::snapshots.filter(snaps_dsl::flow_id.eq(&parent_id_s)
                                                                    .and(snaps_dsl::cursor.le(parent_cursor)))
                                          .load::<SnapshotRow>(conn)?;
          // Get parent flow info
          let parent_flow =
            flows_dsl::flows.select(FlowRow::as_select()).filter(flows_dsl::id.eq(&parent_id_s)).first::<FlowRow>(conn)?;
          let status_in = parent_flow.status;
          let name_in = parent_flow.name.map(|n| format!("{}_branch", n)).or(Some("branch".into()));
          let new = FlowRow { id: new_id.to_string(),
                              name: name_in.clone(),
                              status: status_in.clone(),
                              created_by: None,
                              created_at_ts: now_ts,
                              current_cursor: parent_cursor,
                              current_version: 0,
                              parent_flow_id: Some(parent_flow_id.to_string()),
                              parent_cursor: Some(parent_cursor),
                              metadata: meta_s };
          diesel::insert_into(flows_dsl::flows).values(&new).execute(conn)?;
          for r in rows {
            let copy = FlowDataRow { id: Uuid::new_v4().to_string(),
                                     flow_id: new_id.to_string(),
                                     cursor: r.cursor,
                                     key: r.key.clone(),
                                     payload: r.payload.clone(),
                                     metadata: r.metadata.clone(),
                                     command_id: r.command_id.clone(),
                                     created_at_ts: r.created_at_ts };
            diesel::insert_into(data_dsl::flow_data).values(&copy).execute(conn)?;
          }
          for s in snaps {
            let s_copy = SnapshotRow { id: Uuid::new_v4().to_string(),
                                       flow_id: new_id.to_string(),
                                       cursor: s.cursor,
                                       state_ptr: s.state_ptr.clone(),
                                       metadata: s.metadata.clone(),
                                       created_at_ts: s.created_at_ts };
            diesel::insert_into(snaps_dsl::snapshots).values(&s_copy).execute(conn)?;
          }
          Ok(new_id)
        })
        .map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
  }

  async fn delete_branch(&self, flow_id: &Uuid, recursive: bool) -> FlowResult<()> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();

    if recursive {
      // Find child flows and delete them recursively
      let child_rows: Vec<FlowRow> = map_db_err(flows_dsl::flows.select(FlowRow::as_select())
                                                                .filter(flows_dsl::parent_flow_id.eq(Some(fid.clone())))
                                                                .load::<FlowRow>(&mut conn))?;
      for child in child_rows {
        if let Ok(child_uuid) = Uuid::parse_str(&child.id) {
          BranchManagementPort::delete_branch(self, &child_uuid, true).await?;
        }
      }
    }

    // Delete data, snapshots and flow
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
          diesel::delete(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid))).execute(conn)?;
          diesel::delete(schema::snapshots::dsl::snapshots.filter(schema::snapshots::dsl::flow_id.eq(&fid))).execute(conn)?;
          diesel::delete(flows_dsl::flows.filter(flows_dsl::id.eq(&fid))).execute(conn)?;
          Ok(())
        })
        .map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
  }

  async fn list_child_branches(&self, parent_flow_id: &Uuid) -> FlowResult<Vec<Uuid>> {
    let mut conn = self.conn()?;
    let parent_id_s = parent_flow_id.to_string();
    let rows = map_db_err(flows_dsl::flows.select(flows_dsl::id)
                                          .filter(flows_dsl::parent_flow_id.eq(Some(parent_id_s)))
                                          .load::<String>(&mut conn))?;
    let mut out = Vec::new();
    for s in rows {
      if let Ok(u) = Uuid::parse_str(&s) {
        out.push(u);
      }
    }
    Ok(out)
  }

  async fn get_branch_info(&self, flow_id: &Uuid) -> FlowResult<Option<BranchInfo>> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let row_opt = flows_dsl::flows.select(FlowRow::as_select())
                                  .filter(flows_dsl::id.eq(&fid))
                                  .first::<FlowRow>(&mut conn)
                                  .optional()
                                  .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(row) = row_opt {
      let info = BranchInfo { flow_id: *flow_id,
                              parent_flow_id: row.parent_flow_id.and_then(|s| Uuid::parse_str(&s).ok()),
                              parent_cursor: row.parent_cursor,
                              created_at: Utc.timestamp_opt(row.created_at_ts, 0).single().unwrap_or(Utc::now()),
                              metadata: serde_json::from_str(&row.metadata).unwrap_or(serde_json::json!({})) };
      Ok(Some(info))
    } else {
      Ok(None)
    }
  }

  async fn branch_exists(&self, flow_id: &Uuid) -> FlowResult<bool> {
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let c: i64 = map_db_err(flows_dsl::flows.filter(flows_dsl::id.eq(&fid)).count().get_result(&mut conn))?;
    Ok(c > 0)
  }
}

#[async_trait]
impl SnapshotPort for DieselFlowRepository {
  async fn save_snapshot(&self,
                         flow_id_param: &Uuid,
                         cursor_param: i64,
                         state_ptr_param: &str,
                         metadata_param: JsonValue)
                         -> FlowResult<Uuid> {
    use schema::snapshots::dsl::*;
    let mut conn = self.conn()?;
    let new_id = Uuid::new_v4();
    let now_ts = Utc::now().timestamp();
    let snap = SnapshotRow { id: new_id.to_string(),
                             flow_id: flow_id_param.to_string(),
                             cursor: cursor_param,
                             state_ptr: state_ptr_param.to_string(),
                             metadata: metadata_param.to_string(),
                             created_at_ts: now_ts };
    diesel::insert_into(snapshots).values(&snap).execute(&mut conn).map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    Ok(new_id)
  }

  async fn load_latest_snapshot(&self, flow_id_param: &Uuid) -> FlowResult<Option<SnapshotMeta>> {
    use schema::snapshots::dsl::*;
    let mut conn = self.conn()?;
    let fid_s = flow_id_param.to_string();
    let row_opt = snapshots.filter(flow_id.eq(&fid_s))
                           .order((cursor.desc(), created_at_ts.desc()))
                           .first::<SnapshotRow>(&mut conn)
                           .optional()
                           .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(r) = row_opt {
      let meta = SnapshotMeta { id: Uuid::parse_str(&r.id).unwrap(),
                                flow_id: Uuid::parse_str(&r.flow_id).unwrap(),
                                cursor: r.cursor,
                                state_ptr: r.state_ptr.clone(),
                                metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
                                created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()) };
      Ok(Some(meta))
    } else {
      Ok(None)
    }
  }

  async fn load_snapshot(&self, snapshot_id: &Uuid) -> FlowResult<(Vec<u8>, SnapshotMeta)> {
    use schema::snapshots::dsl::*;
    let mut conn = self.conn()?;
    let sid = snapshot_id.to_string();
    let r =
      snapshots.filter(id.eq(&sid)).first::<SnapshotRow>(&mut conn).map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    let bytes = self.load(&r.state_ptr)?;
    let meta = SnapshotMeta { id: Uuid::parse_str(&r.id).unwrap(),
                              flow_id: Uuid::parse_str(&r.flow_id).unwrap(),
                              cursor: r.cursor,
                              state_ptr: r.state_ptr.clone(),
                              metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
                              created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()) };
    Ok((bytes, meta))
  }

  async fn list_snapshots(&self, flow_id_param: &Uuid) -> FlowResult<Vec<SnapshotMeta>> {
    use schema::snapshots::dsl::*;
    let mut conn = self.conn()?;
    let fid_s = flow_id_param.to_string();
    let rows = map_db_err(snapshots.filter(flow_id.eq(&fid_s)).order(created_at_ts.desc()).load::<SnapshotRow>(&mut conn))?;
    let mut out = Vec::new();
    for r in rows {
      let meta = SnapshotMeta { id: Uuid::parse_str(&r.id).unwrap(),
                                flow_id: Uuid::parse_str(&r.flow_id).unwrap(),
                                cursor: r.cursor,
                                state_ptr: r.state_ptr.clone(),
                                metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
                                created_at: Utc.timestamp_opt(r.created_at_ts, 0).single().unwrap_or(Utc::now()) };
      out.push(meta);
    }
    Ok(out)
  }

  async fn cleanup_old_snapshots(&self, flow_id_param: &Uuid, keep_latest: usize) -> FlowResult<()> {
    use schema::snapshots::dsl::*;
    let mut conn = self.conn()?;
    let fid_s = flow_id_param.to_string();

    // Get all snapshots ordered by creation time desc
    let all_snapshots =
      map_db_err(snapshots.filter(flow_id.eq(&fid_s)).order(created_at_ts.desc()).load::<SnapshotRow>(&mut conn))?;

    if all_snapshots.len() > keep_latest {
      // Delete older snapshots
      let to_delete = &all_snapshots[keep_latest..];
      for snap in to_delete {
        // Also delete the file
        let _ = self.delete_snapshot_file(&snap.state_ptr);
        map_db_err(diesel::delete(snapshots.filter(id.eq(&snap.id))).execute(&mut conn))?;
      }
    }

    Ok(())
  }
}

#[async_trait]
impl MetadataPort for DieselFlowRepository {
  async fn get_metadata(&self, flow_id: &Uuid, key: &str) -> FlowResult<JsonValue> {
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let row = flows.select(metadata)
                   .filter(id.eq(&fid))
                   .first::<String>(&mut conn)
                   .optional()
                   .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(meta_s) = row {
      let meta_json: JsonValue = serde_json::from_str(&meta_s).unwrap_or(serde_json::json!({}));
      Ok(meta_json.get(key).cloned().unwrap_or(JsonValue::Null))
    } else {
      Err(FlowError::NotFound(format!("flow {}", flow_id)))
    }
  }

  async fn set_metadata(&self, flow_id: &Uuid, key: &str, value: JsonValue) -> FlowResult<()> {
    use diesel::prelude::*;
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    // Read current metadata
    let current = flows.select(metadata)
                       .filter(id.eq(&fid))
                       .first::<String>(&mut conn)
                       .optional()
                       .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(mut meta_s) = current {
      let mut meta_json: JsonValue = serde_json::from_str(&meta_s).unwrap_or(serde_json::json!({}));
      if !meta_json.is_object() {
        meta_json = serde_json::json!({});
      }
      if let Some(obj) = meta_json.as_object_mut() {
        obj.insert(key.to_string(), value);
      }
      meta_s = meta_json.to_string();
      map_db_err(diesel::update(flows.filter(id.eq(&fid))).set(metadata.eq(meta_s)).execute(&mut conn))?;
      Ok(())
    } else {
      Err(FlowError::NotFound(format!("flow {}", flow_id)))
    }
  }

  async fn delete_metadata(&self, flow_id: &Uuid, key: &str) -> FlowResult<()> {
    use diesel::prelude::*;
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let current = flows.select(metadata)
                       .filter(id.eq(&fid))
                       .first::<String>(&mut conn)
                       .optional()
                       .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(mut meta_s) = current {
      let mut meta_json: JsonValue = serde_json::from_str(&meta_s).unwrap_or(serde_json::json!({}));
      if let Some(obj) = meta_json.as_object_mut() {
        obj.remove(key);
      }
      meta_s = meta_json.to_string();
      map_db_err(diesel::update(flows.filter(id.eq(&fid))).set(metadata.eq(meta_s)).execute(&mut conn))?;
      Ok(())
    } else {
      Err(FlowError::NotFound(format!("flow {}", flow_id)))
    }
  }

  async fn list_metadata_keys(&self, flow_id: &Uuid) -> FlowResult<Vec<String>> {
    use schema::flows::dsl::*;
    let mut conn = self.conn()?;
    let fid = flow_id.to_string();
    let row = flows.select(metadata)
                   .filter(id.eq(&fid))
                   .first::<String>(&mut conn)
                   .optional()
                   .map_err(|e| FlowError::Storage(format!("db: {}", e)))?;
    if let Some(meta_s) = row {
      let meta_json: JsonValue = serde_json::from_str(&meta_s).unwrap_or(serde_json::json!({}));
      if let Some(obj) = meta_json.as_object() {
        Ok(obj.keys().cloned().collect())
      } else {
        Ok(Vec::new())
      }
    } else {
      Err(FlowError::NotFound(format!("flow {}", flow_id)))
    }
  }
}

#[async_trait]
impl OutboundFlowRepository for DieselFlowRepository {
  async fn create_flow_with_initial_data(&self, metadata: FlowMetadata, initial_data: FlowData) -> FlowResult<Uuid> {
    let mut conn = self.conn()?;
    let flow_id = metadata.id;

    conn.transaction::<Uuid, diesel::result::Error, _>(|conn| {
          // Create flow
          let meta_s = metadata.metadata.to_string();
          let flow_row = FlowRow { id: flow_id.to_string(),
                                   name: metadata.name,
                                   status: metadata.status,
                                   created_by: metadata.created_by,
                                   created_at_ts: metadata.created_at.timestamp(),
                                   current_cursor: metadata.current_cursor,
                                   current_version: metadata.current_version,
                                   parent_flow_id: metadata.parent_flow_id.map(|u| u.to_string()),
                                   parent_cursor: metadata.parent_cursor,
                                   metadata: meta_s };
          diesel::insert_into(flows_dsl::flows).values(&flow_row).execute(conn)?;

          // Insert initial data
          let data_row = FlowDataRow { id: initial_data.id.to_string(),
                                       flow_id: initial_data.flow_id.to_string(),
                                       cursor: initial_data.cursor,
                                       key: initial_data.key.clone(),
                                       payload: initial_data.payload.to_string(),
                                       metadata: initial_data.metadata.to_string(),
                                       command_id: initial_data.command_id.map(|u| u.to_string()),
                                       created_at_ts: initial_data.created_at.timestamp() };
          diesel::insert_into(data_dsl::flow_data).values(&data_row).execute(conn)?;

          Ok(flow_id)
        })
        .map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
  }

  async fn get_repository_stats(&self) -> FlowResult<RepositoryStats> {
    let mut conn = self.conn()?;

    let total_flows = map_db_err(flows_dsl::flows.count().get_result::<i64>(&mut conn))? as usize;
    let total_data_records = map_db_err(data_dsl::flow_data.count().get_result::<i64>(&mut conn))? as usize;
    let total_snapshots = map_db_err(schema::snapshots::dsl::snapshots.count().get_result::<i64>(&mut conn))? as usize;
    let total_branches = map_db_err(flows_dsl::flows.filter(flows_dsl::parent_flow_id.is_not_null())
                                                    .count()
                                                    .get_result::<i64>(&mut conn))? as usize;

    // For storage size, we'd need to calculate file sizes, but for now return 0
    let storage_size_bytes = 0u64;

    Ok(RepositoryStats { total_flows, total_data_records, total_snapshots, total_branches, storage_size_bytes })
  }
}

impl DieselFlowRepository {
  fn delete_snapshot_file(&self, state_ptr: &str) -> FlowResult<()> {
    let path = Path::new(&self.snapshot_dir).join(state_ptr);
    if path.exists() {
      fs::remove_file(&path).map_err(|e| FlowError::Storage(format!("delete snapshot file: {}", e)))?;
    }
    Ok(())
  }
}

impl FlowRepository for DieselFlowRepository {
  fn get_flow_meta(&self, flow_id: &Uuid) -> FlowResult<FlowMeta> {
    futures::executor::block_on(FlowMetadataPort::get_flow_metadata(self, flow_id))
  }

  fn create_flow(&self, name: Option<String>, status: Option<String>, metadata: JsonValue) -> FlowResult<Uuid> {
    let meta = FlowMetadata { id: Uuid::new_v4(),
                              name,
                              status,
                              created_by: None,
                              created_at: Utc::now(),
                              current_cursor: 0,
                              current_version: 0,
                              parent_flow_id: None,
                              parent_cursor: None,
                              metadata };
    futures::executor::block_on(FlowMetadataPort::create_flow(self, meta))
  }

  fn persist_data(&self, data: &FlowData, expected_version: i64) -> FlowResult<PersistResult> {
    futures::executor::block_on(FlowDataPort::persist_data(self, data, expected_version))
  }

  fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> FlowResult<Vec<FlowData>> {
    futures::executor::block_on(FlowDataPort::read_data(self, flow_id, from_cursor))
  }

  fn load_latest_snapshot(&self, flow_id: &Uuid) -> FlowResult<Option<SnapshotMeta>> {
    futures::executor::block_on(SnapshotPort::load_latest_snapshot(self, flow_id))
  }

  fn load_snapshot(&self, snapshot_id: &Uuid) -> FlowResult<(Vec<u8>, SnapshotMeta)> {
    futures::executor::block_on(SnapshotPort::load_snapshot(self, snapshot_id))
  }

  fn save_snapshot(&self, flow_id: &Uuid, cursor: i64, state_ptr: &str, metadata: JsonValue) -> FlowResult<Uuid> {
    futures::executor::block_on(SnapshotPort::save_snapshot(self, flow_id, cursor, state_ptr, metadata))
  }

  fn create_branch(&self, parent_flow_id: &Uuid, parent_cursor: i64, metadata: JsonValue) -> FlowResult<Uuid> {
    futures::executor::block_on(BranchManagementPort::create_branch(self, parent_flow_id, parent_cursor, metadata))
  }

  fn branch_exists(&self, flow_id: &Uuid) -> FlowResult<bool> {
    futures::executor::block_on(BranchManagementPort::branch_exists(self, flow_id))
  }

  fn count_steps(&self, flow_id: &Uuid) -> FlowResult<i64> {
    let meta = futures::executor::block_on(FlowMetadataPort::get_flow_metadata(self, flow_id))?;
    Ok(meta.current_cursor)
  }

  fn delete_branch(&self, flow_id: &Uuid) -> FlowResult<()> {
    // Only delete the branch itself, not recursively
    futures::executor::block_on(BranchManagementPort::delete_branch(self, flow_id, false))
  }

  fn delete_from_step(&self, flow_id: &Uuid, from_cursor: i64) -> FlowResult<()> {
    // Custom logic: delete child branches whose parent_cursor >= from_cursor
    let mut conn = self.conn().map_err(|e| FlowError::Storage(format!("pool: {}", e)))?;
    let fid = flow_id.to_string();
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
      // Delete flow_data with cursor >= from_cursor
      diesel::delete(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&fid).and(data_dsl::cursor.ge(from_cursor)))).execute(conn)?;
      // Delete child branches whose parent_cursor >= from_cursor
      let child_rows: Vec<FlowRow> = flows_dsl::flows
        .select(FlowRow::as_select())
        .filter(flows_dsl::parent_flow_id.eq(Some(fid.clone())))
        .filter(flows_dsl::parent_cursor.ge(from_cursor))
        .load::<FlowRow>(conn)?;
      for child in child_rows {
        diesel::delete(flows_dsl::flows.filter(flows_dsl::id.eq(&child.id))).execute(conn)?;
        diesel::delete(data_dsl::flow_data.filter(data_dsl::flow_id.eq(&child.id))).execute(conn)?;
        diesel::delete(schema::snapshots::dsl::snapshots.filter(schema::snapshots::dsl::flow_id.eq(&child.id))).execute(conn)?;
      }
      // Update current_cursor to max remaining cursor or 0
      let new_cursor_opt: Option<i64> = data_dsl::flow_data
        .filter(data_dsl::flow_id.eq(&fid))
        .select(diesel::dsl::max(data_dsl::cursor))
        .first::<Option<i64>>(conn)?;
      let new_cursor = new_cursor_opt.unwrap_or(0);
      diesel::update(flows_dsl::flows.filter(flows_dsl::id.eq(&fid)))
        .set(flows_dsl::current_cursor.eq(new_cursor))
        .execute(conn)?;
      Ok(())
    }).map_err(|e| FlowError::Storage(format!("db txn: {}", e)))
  }

  fn lock_for_update(&self, _flow_id: &Uuid, _expected_version: i64) -> FlowResult<bool> {
    // Simple optimistic lock: rely on expected_version checks in persist_data
    Ok(true)
  }

  fn claim_work(&self, _worker_id: &str) -> FlowResult<Option<WorkItem>> {
    Ok(None)
  }

  fn get_flow_status(&self, flow_id: &Uuid) -> FlowResult<Option<String>> {
    futures::executor::block_on(FlowMetadataPort::get_flow_status(self, flow_id))
  }

  fn set_flow_status(&self, flow_id: &Uuid, new_status: Option<String>) -> FlowResult<FlowMeta> {
    // Read, modify, update, then return updated metadata
    let mut meta = futures::executor::block_on(FlowMetadataPort::get_flow_metadata(self, flow_id))?;
    meta.status = new_status;
    futures::executor::block_on(FlowMetadataPort::update_flow_metadata(self, flow_id, meta.clone()))?;
    Ok(meta)
  }

  fn get_meta(&self, flow_id: &Uuid, key: &str) -> FlowResult<JsonValue> {
    futures::executor::block_on(MetadataPort::get_metadata(self, flow_id, key))
  }

  fn set_meta(&self, flow_id: &Uuid, key: &str, value: JsonValue) -> FlowResult<()> {
    futures::executor::block_on(MetadataPort::set_metadata(self, flow_id, key, value))
  }

  fn del_meta(&self, flow_id: &Uuid, key: &str) -> FlowResult<()> {
    futures::executor::block_on(MetadataPort::delete_metadata(self, flow_id, key))
  }

  fn list_flow_ids(&self) -> FlowResult<Vec<Uuid>> {
    futures::executor::block_on(FlowMetadataPort::list_flow_ids(self))
  }

  fn dump_tables_for_debug(&self) -> FlowResult<(Vec<FlowMeta>, Vec<FlowData>)> {
    let ids = futures::executor::block_on(FlowMetadataPort::list_flow_ids(self))?;
    let mut metas = Vec::new();
    let mut all_data = Vec::new();
    for id in ids.iter() {
      let meta = futures::executor::block_on(FlowMetadataPort::get_flow_metadata(self, id))?;
      metas.push(meta);
      let data = futures::executor::block_on(FlowDataPort::read_data(self, id, 0))?;
      all_data.extend(data);
    }
    Ok((metas, all_data))
  }
}
impl SnapshotStore for DieselFlowRepository {
  fn save(&self, state: &[u8]) -> FlowResult<String> {
    let key = format!("{}.bin", Uuid::new_v4());
    let path = Path::new(&self.snapshot_dir).join(&key);
    fs::write(&path, state).map_err(|e| FlowError::Storage(e.to_string()))?;
    Ok(key)
  }
  fn load(&self, key: &str) -> FlowResult<Vec<u8>> {
    let path = Path::new(&self.snapshot_dir).join(key);
    fs::read(&path).map_err(|e| FlowError::Storage(e.to_string()))
  }
}
impl ArtifactStore for DieselFlowRepository {
  fn put(&self, blob: &[u8]) -> FlowResult<String> {
    let key = format!("{}.bin", Uuid::new_v4());
    let path = Path::new(&self.artifact_dir).join(&key);
    fs::write(&path, blob).map_err(|e| FlowError::Storage(e.to_string()))?;
    Ok(key)
  }
  fn get(&self, key: &str) -> FlowResult<Vec<u8>> {
    let path = Path::new(&self.artifact_dir).join(key);
    fs::read(&path).map_err(|e| FlowError::Storage(e.to_string()))
  }
  fn copy_if_needed(&self, src_key: &str) -> FlowResult<String> {
    let new_key = format!("{}.bin", Uuid::new_v4());
    let src_path = Path::new(&self.artifact_dir).join(src_key);
    let dest_path = Path::new(&self.artifact_dir).join(&new_key);
    fs::copy(&src_path, &dest_path).map_err(|e| FlowError::Storage(e.to_string()))?;
    Ok(new_key)
  }
}
