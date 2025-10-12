// repository.rs
use crate::db::run_migrations_on_pool;
use crate::schema;
use crate::schema::families::dsl as families_dsl;
use crate::schema::family_members::dsl as fm_dsl;
use crate::schema::molecules::dsl as molecules_dsl;
use chem_domain::{DomainError, Molecule, MoleculeFamily};
use chem_domain::ports::{FamilyRepository, MoleculeReader, MoleculeWriter, PropertyRepository};
use chem_domain::{OwnedFamilyProperty, OwnedMolecularProperty};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error as DieselError;
// ...existing code...
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "postgres")]
type DbPool = Pool<ConnectionManager<PgConnection>>;
#[cfg(not(feature = "postgres"))]
type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Repo Diesel que implementa los ports de dominio.
pub struct DieselDomainRepository {
  pool: Arc<DbPool>,
}

// Convenience constructors exposed by the crate root (lib.rs)
#[cfg(all(feature = "postgres", not(test)))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url = std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                                      DomainError::persistence("database",
                                                                                                       "CHEM_DB_URL/\
                                                                                                        DATABASE_URL not \
                                                                                                        set"
                                                                                                            .to_string())
                                                                                    })?;
  if !(url.starts_with("postgres") || url.starts_with("postgresql://") || url.contains("@")) {
    return Err(DomainError::persistence("database",
                                        "chem-persistence: CHEM_DB_URL does not look like Postgres URL".to_string()));
  }
  Ok(DieselDomainRepository::new(&url))
}

#[cfg(all(test, feature = "sqlite", not(feature = "postgres")))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  use crate::test_helpers::{create_temp_sqlite_db, TestSqliteDb};
  use once_cell::sync::Lazy;
  use std::sync::Mutex;

  static TEST_DBS: Lazy<Mutex<Vec<TestSqliteDb>>> = Lazy::new(|| Mutex::new(Vec::new()));

  let db = create_temp_sqlite_db().map_err(|e| DomainError::persistence("database", format!("sqlite helper: {}", e)))?;
  let repo = DieselDomainRepository::new_with_pool(db.pool.clone())?;
  TEST_DBS.lock().expect("poisoned sqlite test db mutex").push(db);
  Ok(repo)
}

#[cfg(all(test, feature = "postgres"))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url = std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                                      DomainError::persistence("database",
                                                                                                       "CHEM_DB_URL/\
                                                                                                        DATABASE_URL not \
                                                                                                        set"
                                                                                                            .to_string())
                                                                                    })?;
  Ok(DieselDomainRepository::new(&url))
}

#[cfg(all(not(feature = "postgres"), not(test)))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url = std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                                      DomainError::persistence("database",
                                                                                                       "CHEM_DB_URL/\
                                                                                                        DATABASE_URL not \
                                                                                                        set"
                                                                                                            .to_string())
                                                                                    })?;
  let url_l = url.to_lowercase();
  if url_l.contains("mode=memory") {
    return Err(DomainError::persistence("database",
                                        "chem-persistence: in-memory sqlite URLs are not supported; provide a \
                                         file-backed path"
                                                          .to_string()));
  }
  if url_l.starts_with("file:") || url_l.contains("sqlite") {
    return Ok(DieselDomainRepository::new(&url));
  }
  Err(DomainError::persistence("database", "chem-persistence was compiled without 'pg' feature; enable 'pg' to use Postgres".to_string()))
}

#[cfg(all(test, not(feature = "sqlite"), not(feature = "postgres")))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url =
    std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                              DomainError::ExternalError("CHEM_DB_URL/\
                                                                                                          DATABASE_URL not \
                                                                                                          set"
                                                                                                              .to_string())
                                                                            })?;
  let url_l = url.to_lowercase();
  if url_l.contains("mode=memory") {
    return Err(DomainError::ExternalError("chem-persistence: in-memory sqlite URLs are not supported; provide a \
                                           file-backed path"
                                                            .to_string()));
  }
  if url_l.starts_with("file:") || url_l.contains("sqlite") {
    return Ok(DieselDomainRepository::new(&url));
  }
  Err(DomainError::persistence("database",
                               "chem-persistence requires either the 'postgres' or 'sqlite' feature".to_string()))
}

/// Helper that returns a repo created from environment and boxed as trait
/// object
pub fn new_domain_repo_from_env() -> Result<DieselDomainRepository, DomainError> {
  new_from_env()
}

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub fn new_sqlite_for_test() -> Result<(DieselDomainRepository, crate::test_helpers::TestSqliteDb), DomainError> {
  use crate::test_helpers::{create_temp_sqlite_db, TestSqliteDb};

  let db: TestSqliteDb =
    create_temp_sqlite_db().map_err(|e| DomainError::persistence("database", format!("sqlite helper: {}", e)))?;
  let repo = DieselDomainRepository::new_with_pool(db.pool.clone())?;
  Ok((repo, db))
}

impl DieselDomainRepository {
  pub fn new(database_url: &str) -> Self {
    #[cfg(not(feature = "postgres"))]
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    #[cfg(feature = "postgres")]
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().max_size(4).build(manager).expect("no se pudo crear el pool de conexiones");
    DieselDomainRepository::new_with_pool(pool).expect("failed to initialize DieselDomainRepository")
  }

  pub fn new_with_pool(pool: DbPool) -> Result<Self, DomainError> {
    run_migrations_on_pool(&pool).map_err(|e| DomainError::persistence("database", format!("migrations: {}", e)))?;
    Ok(DieselDomainRepository { pool: Arc::new(pool) })
  }

  pub fn pool(&self) -> Arc<DbPool> {
    Arc::clone(&self.pool)
  }
}

// Diesel row structs for the chemical tables
#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = schema::molecules)]
struct MoleculeRow {
  pub inchikey: String,
  pub smiles: String,
  pub inchi: String,
  pub metadata: String,
  pub structure: Option<String>,
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = schema::families)]
struct FamilyRow {
  pub id: String,
  pub name: Option<String>,
  pub description: Option<String>,
  pub family_hash: String,
  pub provenance: String,
  pub frozen: bool,
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = schema::family_properties)]
struct FamilyPropertyRow {
  pub id: String,
  pub family_id: String,
  pub property_type: String,
  pub value: String,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  pub metadata: String,
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = schema::molecular_properties)]
struct MolecularPropertyRow {
  pub id: String,
  pub molecule_inchikey: String,
  pub property_type: String,
  pub value: String,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  pub metadata: String,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = schema::family_members)]
struct FamilyMemberRow {
  pub id: String,
  pub family_id: String,
  pub molecule_inchikey: String,
}

// ============================================================================
// PHASE 4: Port Implementations for DieselDomainRepository
// ============================================================================

// Helper function to map Diesel errors to DomainError
fn map_db_err<T>(res: std::result::Result<T, DieselError>) -> Result<T, DomainError> {
  res.map_err(|e| DomainError::persistence("database", format!("db: {}", e)))
}

// Implementation of MoleculeReader port
impl MoleculeReader for DieselDomainRepository {
  fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let opt = molecules_dsl::molecules
      .filter(molecules_dsl::inchikey.eq(inchikey))
      .first::<MoleculeRow>(&mut conn)
      .optional()
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;
    
    if let Some(r) = opt {
      let mut metadata_val = serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({}));
      if let Some(s) = r.structure.as_ref() {
        if let Ok(struct_json) = serde_json::from_str::<serde_json::Value>(s) {
          metadata_val["structure"] = struct_json;
        }
      }
      let mol = Molecule::from_parts(&r.inchikey, &r.smiles, &r.inchi, metadata_val)?;
      Ok(Some(mol))
    } else {
      Ok(None)
    }
  }

  fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let rows = molecules_dsl::molecules
      .load::<MoleculeRow>(&mut conn)
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;
    
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
      let mut metadata_val = serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({}));
      if let Some(s) = r.structure.as_ref() {
        if let Ok(struct_json) = serde_json::from_str::<serde_json::Value>(s) {
          metadata_val["structure"] = struct_json;
        }
      }
      let mol = Molecule::from_parts(&r.inchikey, &r.smiles, &r.inchi, metadata_val)?;
      out.push(mol);
    }
    Ok(out)
  }

  fn find_by_smiles(&self, smiles: &str) -> Result<Vec<Molecule>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let rows = molecules_dsl::molecules
      .filter(molecules_dsl::smiles.eq(smiles))
      .load::<MoleculeRow>(&mut conn)
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;
    
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
      let mut metadata_val = serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({}));
      if let Some(s) = r.structure.as_ref() {
        if let Ok(struct_json) = serde_json::from_str::<serde_json::Value>(s) {
          metadata_val["structure"] = struct_json;
        }
      }
      let mol = Molecule::from_parts(&r.inchikey, &r.smiles, &r.inchi, metadata_val)?;
      out.push(mol);
    }
    Ok(out)
  }
}

// Implementation of MoleculeWriter port
impl MoleculeWriter for DieselDomainRepository {
  fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let mr = MoleculeRow {
      inchikey: molecule.inchikey().to_string(),
      smiles: molecule.smiles().to_string(),
      inchi: molecule.inchi().to_string(),
      metadata: molecule.metadata().to_string(),
      structure: molecule.metadata().get("structure").and_then(|v| serde_json::to_string(v).ok()),
    };

    #[cfg(feature = "postgres")]
    {
      map_db_err(
        diesel::insert_into(schema::molecules::table)
          .values(&mr)
          .on_conflict(schema::molecules::inchikey)
          .do_nothing()
          .execute(&mut conn),
      )?;
    }
    #[cfg(not(feature = "postgres"))]
    {
      let res = diesel::sql_query(
        "INSERT OR IGNORE INTO molecules (inchikey, smiles, inchi, metadata, structure) VALUES (?, ?, ?, ?, ?)",
      )
      .bind::<diesel::sql_types::Text, _>(mr.inchikey.clone())
      .bind::<diesel::sql_types::Text, _>(mr.smiles)
      .bind::<diesel::sql_types::Text, _>(mr.inchi)
      .bind::<diesel::sql_types::Text, _>(mr.metadata)
      .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(mr.structure.clone())
      .execute(&mut conn);
      map_db_err(res)?;
    }

    Ok(molecule.inchikey().to_string())
  }

  fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let tx_result = conn.transaction::<(), diesel::result::Error, _>(|conn| {
      // Check if referenced
      let exists = fm_dsl::family_members
        .filter(fm_dsl::molecule_inchikey.eq(inchikey))
        .select(fm_dsl::id)
        .first::<String>(conn)
        .optional()?;
      
      if exists.is_some() {
        return Err(diesel::result::Error::RollbackTransaction);
      }
      
      diesel::delete(molecules_dsl::molecules.filter(molecules_dsl::inchikey.eq(inchikey)))
        .execute(conn)?;
      Ok(())
    });

    match tx_result {
      Ok(_) => Ok(()),
      Err(diesel::result::Error::RollbackTransaction) => {
        Err(DomainError::validation(
          "Molecule",
          format!("No se puede eliminar la molecula {}; pertenece a una familia", inchikey),
        ))
      }
      Err(e) => Err(DomainError::persistence("database", format!("db: {}", e))),
    }
  }
}

// Implementation of FamilyRepository port
impl FamilyRepository for DieselDomainRepository {
  fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    conn.transaction::<Uuid, diesel::result::Error, _>(|conn| {
      let id_s = family.id().to_string();
      let family_row = FamilyRow {
        id: id_s.clone(),
        name: family.name().map(|s| s.to_string()),
        description: family.description().map(|s| s.to_string()),
        family_hash: family.family_hash().to_string(),
        provenance: family.provenance().to_string(),
        frozen: family.is_frozen(),
      };

      // Insert family
      diesel::insert_into(schema::families::table)
        .values(&family_row)
        .execute(conn)?;

      // Insert molecules with on_conflict_do_nothing
      for m in family.molecules() {
        let mr = MoleculeRow {
          inchikey: m.inchikey().to_string(),
          smiles: m.smiles().to_string(),
          inchi: m.inchi().to_string(),
          metadata: m.metadata().to_string(),
          structure: m.metadata().get("structure").and_then(|v| serde_json::to_string(v).ok()),
        };
        
        #[cfg(feature = "postgres")]
        {
          let _ = diesel::insert_into(schema::molecules::table)
            .values(&mr)
            .on_conflict(schema::molecules::inchikey)
            .do_nothing()
            .execute(conn);
        }
        #[cfg(not(feature = "postgres"))]
        {
          let _ = diesel::sql_query(
            "INSERT OR IGNORE INTO molecules (inchikey, smiles, inchi, metadata, structure) VALUES (?, ?, ?, ?, ?)",
          )
          .bind::<diesel::sql_types::Text, _>(mr.inchikey)
          .bind::<diesel::sql_types::Text, _>(mr.smiles)
          .bind::<diesel::sql_types::Text, _>(mr.inchi)
          .bind::<diesel::sql_types::Text, _>(mr.metadata)
          .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(mr.structure)
          .execute(conn);
        }
      }

      // Insert family members
      for m in family.molecules() {
        let fm = FamilyMemberRow {
          id: Uuid::new_v4().to_string(),
          family_id: id_s.clone(),
          molecule_inchikey: m.inchikey().to_string(),
        };
        diesel::insert_into(schema::family_members::table)
          .values(&fm)
          .execute(conn)?;
      }

      Uuid::parse_str(&id_s).map_err(|_| diesel::result::Error::RollbackTransaction)
    })
    .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))
  }

  fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let id_s = id.to_string();
    let opt = families_dsl::families
      .filter(families_dsl::id.eq(&id_s))
      .first::<FamilyRow>(&mut conn)
      .optional()
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;

    if let Some(r) = opt {
      // Load member inchikeys
      let inchikeys: Vec<String> = fm_dsl::family_members
        .filter(fm_dsl::family_id.eq(&id_s))
        .select(fm_dsl::molecule_inchikey)
        .load(&mut conn)
        .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;

      // Load molecules
      let molecule_rows: Vec<MoleculeRow> = molecules_dsl::molecules
        .filter(molecules_dsl::inchikey.eq_any(&inchikeys))
        .load(&mut conn)
        .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;

      let mut mols = Vec::with_capacity(molecule_rows.len());
      for mr in molecule_rows {
        let mut metadata_val = serde_json::from_str(&mr.metadata).unwrap_or(serde_json::json!({}));
        if let Some(s) = mr.structure.as_ref() {
          if let Ok(struct_json) = serde_json::from_str::<serde_json::Value>(s) {
            metadata_val["structure"] = struct_json;
          }
        }
        let mol = Molecule::from_parts(&mr.inchikey, &mr.smiles, &mr.inchi, metadata_val)?;
        mols.push(mol);
      }

      let provenance = serde_json::from_str(&r.provenance).unwrap_or(serde_json::json!({}));
      let mut mf = MoleculeFamily::new(mols, provenance)?;
      if let Some(n) = r.name {
        mf = mf.with_name(n);
      }
      if let Some(d) = r.description {
        mf = mf.with_description(d);
      }
      let db_id = Uuid::parse_str(&r.id)
        .map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))?;
      mf = mf.with_id(db_id);
      Ok(Some(mf))
    } else {
      Ok(None)
    }
  }

  fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    // Load all families
    let family_rows = families_dsl::families
      .load::<FamilyRow>(&mut conn)
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;

    // Load all family members
    let member_rows = fm_dsl::family_members
      .load::<FamilyMemberRow>(&mut conn)
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;

    // Group members by family_id
    let mut members_by_family: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for mem in member_rows {
      members_by_family
        .entry(mem.family_id)
        .or_default()
        .push(mem.molecule_inchikey);
    }

    // Collect all unique inchikeys
    let all_inchikeys: Vec<String> = members_by_family
      .values()
      .flat_map(|v| v.iter().cloned())
      .collect::<std::collections::HashSet<_>>()
      .into_iter()
      .collect();

    let molecule_rows = if !all_inchikeys.is_empty() {
      molecules_dsl::molecules
        .filter(molecules_dsl::inchikey.eq_any(&all_inchikeys))
        .load::<MoleculeRow>(&mut conn)
        .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?
    } else {
      Vec::new()
    };

    // Map molecules by inchikey
    let mut molecules_by_inchikey: std::collections::HashMap<String, Molecule> = std::collections::HashMap::new();
    for mr in molecule_rows {
      let mut metadata_val = serde_json::from_str(&mr.metadata).unwrap_or(serde_json::json!({}));
      if let Some(s) = mr.structure.as_ref() {
        if let Ok(struct_json) = serde_json::from_str::<serde_json::Value>(s) {
          metadata_val["structure"] = struct_json;
        }
      }
      let mol = Molecule::from_parts(&mr.inchikey, &mr.smiles, &mr.inchi, metadata_val)?;
      molecules_by_inchikey.insert(mr.inchikey, mol);
    }

    // Assemble families
    let mut out = Vec::with_capacity(family_rows.len());
    for r in family_rows {
      let id_s = r.id.clone();
      let inchikeys = members_by_family.get(&id_s).cloned().unwrap_or_default();
      let mut mols = Vec::with_capacity(inchikeys.len());
      for ik in inchikeys {
        if let Some(mol) = molecules_by_inchikey.get(&ik).cloned() {
          mols.push(mol);
        }
      }
      let provenance = serde_json::from_str(&r.provenance).unwrap_or(serde_json::json!({}));
      let mut mf = MoleculeFamily::new(mols, provenance)?;
      if let Some(n) = r.name {
        mf = mf.with_name(n);
      }
      if let Some(d) = r.description {
        mf = mf.with_description(d);
      }
      let db_id = Uuid::parse_str(&r.id)
        .map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))?;
      mf = mf.with_id(db_id);
      out.push(mf);
    }
    Ok(out)
  }

  fn delete_family(&self, id: &Uuid) -> Result<(), DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let id_s = id.to_string();
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
      diesel::delete(fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))).execute(conn)?;
      diesel::delete(families_dsl::families.filter(families_dsl::id.eq(&id_s))).execute(conn)?;
      Ok(())
    })
    .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))
  }

  fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    conn.transaction(|conn| {
      // Load existing family
      let fam_opt = self.get_family(family_id).map_err(|_| diesel::result::Error::RollbackTransaction)?;
      let fam = fam_opt.ok_or_else(|| diesel::result::Error::RollbackTransaction)?;
      let new_fam = fam
        .add_molecule(molecule.clone())
        .map_err(|_| diesel::result::Error::RollbackTransaction)?;

      // Insert molecule if not exists
      let mr = MoleculeRow {
        inchikey: molecule.inchikey().to_string(),
        smiles: molecule.smiles().to_string(),
        inchi: molecule.inchi().to_string(),
        metadata: molecule.metadata().to_string(),
        structure: molecule.metadata().get("structure").and_then(|v| serde_json::to_string(v).ok()),
      };
      
      #[cfg(feature = "postgres")]
      {
        let _ = diesel::insert_into(schema::molecules::table)
          .values(&mr)
          .on_conflict(schema::molecules::inchikey)
          .do_nothing()
          .execute(conn);
      }
      #[cfg(not(feature = "postgres"))]
      {
        let _ = diesel::sql_query(
          "INSERT OR IGNORE INTO molecules (inchikey, smiles, inchi, metadata, structure) VALUES (?, ?, ?, ?, ?)",
        )
        .bind::<diesel::sql_types::Text, _>(mr.inchikey)
        .bind::<diesel::sql_types::Text, _>(mr.smiles)
        .bind::<diesel::sql_types::Text, _>(mr.inchi)
        .bind::<diesel::sql_types::Text, _>(mr.metadata)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(mr.structure)
        .execute(conn);
      }

      // Persist new family
      let new_id_s = new_fam.id().to_string();
      let family_row = FamilyRow {
        id: new_id_s.clone(),
        name: new_fam.name().map(|s| s.to_string()),
        description: new_fam.description().map(|s| s.to_string()),
        family_hash: new_fam.family_hash().to_string(),
        provenance: new_fam.provenance().to_string(),
        frozen: new_fam.is_frozen(),
      };
      diesel::insert_into(schema::families::table)
        .values(&family_row)
        .execute(conn)?;

      for m in new_fam.molecules() {
        let fm = FamilyMemberRow {
          id: Uuid::new_v4().to_string(),
          family_id: new_id_s.clone(),
          molecule_inchikey: m.inchikey().to_string(),
        };
        diesel::insert_into(schema::family_members::table)
          .values(&fm)
          .execute(conn)?;
      }

      Ok(new_fam.id())
    })
    .map_err(|e: diesel::result::Error| DomainError::persistence("database", format!("db: {}", e)))
  }

  fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    conn.transaction(|conn| {
      let fam_opt = self.get_family(family_id).map_err(|_| diesel::result::Error::RollbackTransaction)?;
      let fam = fam_opt.ok_or_else(|| diesel::result::Error::RollbackTransaction)?;
      let new_fam = fam
        .remove_molecule(inchikey)
        .map_err(|_| diesel::result::Error::RollbackTransaction)?;

      // Persist new family
      let new_id_s = new_fam.id().to_string();
      let family_row = FamilyRow {
        id: new_id_s.clone(),
        name: new_fam.name().map(|s| s.to_string()),
        description: new_fam.description().map(|s| s.to_string()),
        family_hash: new_fam.family_hash().to_string(),
        provenance: new_fam.provenance().to_string(),
        frozen: new_fam.is_frozen(),
      };
      diesel::insert_into(schema::families::table)
        .values(&family_row)
        .execute(conn)?;

      for m in new_fam.molecules() {
        let fm = FamilyMemberRow {
          id: Uuid::new_v4().to_string(),
          family_id: new_id_s.clone(),
          molecule_inchikey: m.inchikey().to_string(),
        };
        diesel::insert_into(schema::family_members::table)
          .values(&fm)
          .execute(conn)?;
      }

      Ok(new_fam.id())
    })
    .map_err(|e: diesel::result::Error| match e {
      diesel::result::Error::RollbackTransaction => DomainError::not_found("Family", "not found"),
      other => DomainError::persistence("database", format!("db: {}", other)),
    })
  }
}

// Implementation of PropertyRepository port
impl PropertyRepository for DieselDomainRepository {
  fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let row = FamilyPropertyRow {
      id: prop.id.to_string(),
      family_id: prop.family_id.to_string(),
      property_type: prop.property_type,
      value: prop.value.to_string(),
      quality: prop.quality,
      preferred: prop.preferred,
      value_hash: prop.value_hash,
      metadata: prop.metadata.to_string(),
    };
    
    map_db_err(diesel::insert_into(schema::family_properties::table).values(&row).execute(&mut conn))?;
    Uuid::parse_str(&row.id).map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))
  }

  fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let f_id = family_id.to_string();
    let rows = schema::family_properties::dsl::family_properties
      .filter(schema::family_properties::dsl::family_id.eq(&f_id))
      .load::<FamilyPropertyRow>(&mut conn)
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;
    
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
      out.push(OwnedFamilyProperty {
        id: Uuid::parse_str(&r.id)
          .map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))?,
        family_id: Uuid::parse_str(&r.family_id)
          .map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))?,
        property_type: r.property_type,
        value: serde_json::from_str(&r.value).unwrap_or(serde_json::json!({})),
        quality: r.quality,
        preferred: r.preferred,
        value_hash: r.value_hash,
        metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
      });
    }
    Ok(out)
  }

  fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let row = MolecularPropertyRow {
      id: prop.id.to_string(),
      molecule_inchikey: prop.molecule_inchikey,
      property_type: prop.property_type,
      value: prop.value.to_string(),
      quality: prop.quality,
      preferred: prop.preferred,
      value_hash: prop.value_hash,
      metadata: prop.metadata.to_string(),
    };
    
    map_db_err(diesel::insert_into(schema::molecular_properties::table).values(&row).execute(&mut conn))?;
    Uuid::parse_str(&row.id).map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))
  }

  fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError> {
    let mut conn = self.pool.get().map_err(|e| DomainError::persistence("database", format!("pool: {}", e)))?;
    
    let rows = schema::molecular_properties::dsl::molecular_properties
      .filter(schema::molecular_properties::dsl::molecule_inchikey.eq(inchikey))
      .load::<MolecularPropertyRow>(&mut conn)
      .map_err(|e| DomainError::persistence("database", format!("db: {}", e)))?;
    
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
      out.push(OwnedMolecularProperty {
        id: Uuid::parse_str(&r.id)
          .map_err(|e| DomainError::persistence("database", format!("invalid uuid: {}", e)))?,
        molecule_inchikey: r.molecule_inchikey,
        property_type: r.property_type,
        value: serde_json::from_str(&r.value).unwrap_or(serde_json::json!({})),
        quality: r.quality,
        preferred: r.preferred,
        value_hash: r.value_hash,
        metadata: serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})),
      });
    }
    Ok(out)
  }
}
