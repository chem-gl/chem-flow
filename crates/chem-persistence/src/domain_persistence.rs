use crate::schema;
use crate::schema::families::dsl as families_dsl;
use crate::schema::family_members::dsl as fm_dsl;
use crate::schema::molecular_properties::dsl as molecular_properties_dsl;
use crate::schema::molecules::dsl as mol_dsl;
use crate::schema::molecules::dsl as molecules_dsl;
use chem_domain::{DomainError, DomainRepository, Molecule, MoleculeFamily};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::result::Error as DieselError;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::sync::Arc;
use uuid::Uuid;
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
#[cfg(all(feature = "pg", not(test)))]
type DbPool = Pool<ConnectionManager<PgConnection>>;
#[cfg(any(test, not(feature = "pg")))]
type DbPool = Pool<ConnectionManager<SqliteConnection>>;
#[cfg(all(feature = "pg", not(test)))]
type DbConn = PgConnection;
#[cfg(any(test, not(feature = "pg")))]
type DbConn = SqliteConnection;
/// Repo Diesel que implementa `DomainRepository`.
pub struct DieselDomainRepository {
  pool: Arc<DbPool>,
}
impl DieselDomainRepository {
  pub fn new(database_url: &str) -> Self {
    #[cfg(any(test, not(feature = "pg")))]
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    #[cfg(all(feature = "pg", not(test)))]
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().max_size(4).build(manager).expect("no se pudo crear el pool de conexiones");
    let repo = DieselDomainRepository { pool: Arc::new(pool) };
    if let Ok(mut c) = repo.conn_raw() {
      let _ = diesel::sql_query("PRAGMA journal_mode = WAL;").execute(&mut c);
      let _ = diesel::sql_query("PRAGMA busy_timeout = 5000;").execute(&mut c);
      let _ = c.run_pending_migrations(MIGRATIONS);
    }
    repo
  }
  fn conn_raw(&self) -> std::result::Result<PooledConnection<ConnectionManager<DbConn>>, r2d2::Error> {
    // Note: when built with pg feature this will be adjusted by cfg above
    self.pool.get()
  }
  fn conn(&self) -> Result<PooledConnection<ConnectionManager<DbConn>>, DomainError> {
    self.conn_raw().map_err(|e| DomainError::ExternalError(format!("pool: {}", e)))
  }
}
// Diesel row structs for the chemical tables
#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = schema::molecules)]
struct MoleculeRow {
  pub inchikey: String,
  pub smiles: String,
  pub inchi: String,
  pub metadata: String,
}
#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = schema::families)]
struct FamilyRow {
  pub id: String,
  pub name: Option<String>,
  pub description: Option<String>,
  pub family_hash: String,
  pub provenance: String,
  pub frozen: bool,
}
#[derive(Debug, Queryable, Insertable)]
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
#[derive(Debug, Queryable, Insertable)]
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
fn map_db_err<T>(res: std::result::Result<T, DieselError>) -> Result<T, DomainError> {
  res.map_err(|e| DomainError::ExternalError(format!("db: {}", e)))
}
impl DomainRepository for DieselDomainRepository {
  fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError> {
    use schema::families::dsl::*;
    let mut conn = self.conn()?;
    let id_s = family.id().to_string();
    let row = FamilyRow { id: id_s.clone(),
                          name: family.name().map(|s| s.to_string()),
                          description: family.description().map(|s| s.to_string()),
                          family_hash: family.family_hash().to_string(),
                          provenance: family.provenance().to_string(),
                          frozen: family.is_frozen() };
    // Upsert family row: try insert, on conflict replace (works for sqlite/postgres
    // differently) For simplicity attempt insert, if it errors try
    // delete+insert to replace
    if diesel::insert_into(families).values(&row).execute(&mut conn).is_err() {
      // Try replace: delete existing and insert
      let _ = diesel::delete(families.filter(id.eq(&id_s))).execute(&mut conn);
      map_db_err(diesel::insert_into(families).values(&row).execute(&mut conn))?;
    }
    let _ = diesel::delete(fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))).execute(&mut conn);
    for m in family.molecules().iter() {
      let mr = MoleculeRow { inchikey: m.inchikey().to_string(),
                             smiles: m.smiles().to_string(),
                             inchi: m.inchi().to_string(),
                             metadata: m.metadata().to_string() };
      let _ = diesel::insert_into(mol_dsl::molecules).values(&mr).execute(&mut conn);
      let fm = FamilyMemberRow { id: Uuid::new_v4().to_string(),
                                 family_id: id_s.clone(),
                                 molecule_inchikey: m.inchikey().to_string() };
      let _ = diesel::insert_into(fm_dsl::family_members).values(&fm).execute(&mut conn);
    }
    Ok(Uuid::parse_str(&id_s).unwrap())
  }
  fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError> {
    let mut conn = self.conn()?;
    let id_s = id.to_string();
    let opt = families_dsl::families.filter(families_dsl::id.eq(&id_s))
                                    .first::<FamilyRow>(&mut conn)
                                    .optional()
                                    .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    if let Some(r) = opt {
      // load member inchikeys
      let members = fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))
                                          .load::<FamilyMemberRow>(&mut conn)
                                          .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
      let mut mols = Vec::new();
      for mem in members {
        if let Ok(Some(mr)) =
          mol_dsl::molecules.filter(mol_dsl::inchikey.eq(&mem.molecule_inchikey)).first::<MoleculeRow>(&mut conn).optional()
        {
          let mol = Molecule::from_parts(&mr.inchikey,
                                         &mr.smiles,
                                         &mr.inchi,
                                         serde_json::from_str(&mr.metadata).unwrap_or(serde_json::json!({})))?;
          mols.push(mol);
        }
      }
      let base = MoleculeFamily::new(mols, serde_json::from_str(&r.provenance).unwrap_or(serde_json::json!({})))?;
      let mut mf = base;
      if let Some(n) = r.name.clone() {
        mf = mf.with_name(n)?;
      }
      if let Some(d) = r.description.clone() {
        mf = mf.with_description(d)?;
      }
      // Ensure the returned family keeps the database id
      let db_id = Uuid::parse_str(&r.id).map_err(|e| DomainError::ExternalError(format!("invalid uuid: {}", e)))?;
      mf = mf.with_id(db_id)?;
      Ok(Some(mf))
    } else {
      Ok(None)
    }
  }
  fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError> {
    let mut conn = self.conn()?;
    let mr = MoleculeRow { inchikey: molecule.inchikey().to_string(),
                           smiles: molecule.smiles().to_string(),
                           inchi: molecule.inchi().to_string(),
                           metadata: molecule.metadata().to_string() };
    map_db_err(diesel::insert_into(schema::molecules::table).values(&mr).execute(&mut conn))?;
    Ok(molecule.inchikey().to_string())
  }
  fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError> {
    let mut conn = self.conn()?;
    let opt = molecules_dsl::molecules.filter(molecules_dsl::inchikey.eq(inchikey))
                                      .first::<MoleculeRow>(&mut conn)
                                      .optional()
                                      .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    if let Some(r) = opt {
      let mol = Molecule::from_parts(&r.inchikey,
                                     &r.smiles,
                                     &r.inchi,
                                     serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})))?;
      Ok(Some(mol))
    } else {
      Ok(None)
    }
  }
  fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError> {
    let mut conn = self.conn()?;
    let rows =
      mol_dsl::molecules.load::<MoleculeRow>(&mut conn).map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
      let mol = Molecule::from_parts(&r.inchikey,
                                     &r.smiles,
                                     &r.inchi,
                                     serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})))?;
      out.push(mol);
    }
    Ok(out)
  }
  fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError> {
    let mut conn = self.conn()?;
    let rows =
      families_dsl::families.load::<FamilyRow>(&mut conn).map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let mut out = Vec::new();
    for r in rows {
      let id_s = r.id.clone();
      let members = fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))
                                          .load::<FamilyMemberRow>(&mut conn)
                                          .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
      let mut mols = Vec::new();
      for mem in members {
        if let Ok(Some(mr)) =
          mol_dsl::molecules.filter(mol_dsl::inchikey.eq(&mem.molecule_inchikey)).first::<MoleculeRow>(&mut conn).optional()
        {
          let mol = Molecule::from_parts(&mr.inchikey,
                                         &mr.smiles,
                                         &mr.inchi,
                                         serde_json::from_str(&mr.metadata).unwrap_or(serde_json::json!({})))?;
          mols.push(mol);
        }
      }
      let base = MoleculeFamily::new(mols, serde_json::from_str(&r.provenance).unwrap_or(serde_json::json!({})))?;
      let mut mf = base;
      if let Some(n) = r.name.clone() {
        mf = mf.with_name(n)?;
      }
      if let Some(d) = r.description.clone() {
        mf = mf.with_description(d)?;
      }
      // Preserve DB id on the reconstructed family
      let db_id = Uuid::parse_str(&r.id).map_err(|e| DomainError::ExternalError(format!("invalid uuid: {}", e)))?;
      mf = mf.with_id(db_id)?;
      out.push(mf);
    }
    Ok(out)
  }
  fn save_family_property(&self, prop: chem_domain::OwnedFamilyProperty) -> Result<Uuid, DomainError> {
    let mut conn = self.conn()?;
    let row = FamilyPropertyRow { id: prop.id.to_string(),
                                  family_id: prop.family_id.to_string(),
                                  property_type: prop.property_type,
                                  value: prop.value.to_string(),
                                  quality: prop.quality,
                                  preferred: prop.preferred,
                                  value_hash: prop.value_hash,
                                  metadata: prop.metadata.to_string() };
    map_db_err(diesel::insert_into(schema::family_properties::table).values(&row).execute(&mut conn))?;
    Ok(Uuid::parse_str(&row.id).unwrap())
  }
  fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<chem_domain::OwnedFamilyProperty>, DomainError> {
    let mut conn = self.conn()?;
    use crate::schema::family_properties::dsl as family_properties_dsl;
    let f_id = family_id.to_string();
    let rows = family_properties_dsl::family_properties.filter(family_properties_dsl::family_id.eq(&f_id))
                                                       .load::<FamilyPropertyRow>(&mut conn)
                                                       .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let mut out = Vec::new();
    for r in rows {
      out.push(chem_domain::OwnedFamilyProperty { id: Uuid::parse_str(&r.id).unwrap(),
                                                  family_id: Uuid::parse_str(&r.family_id).unwrap(),
                                                  property_type: r.property_type,
                                                  value: serde_json::from_str(&r.value).unwrap_or(serde_json::json!({})),
                                                  quality: r.quality,
                                                  preferred: r.preferred,
                                                  value_hash: r.value_hash,
                                                  metadata:
                                                    serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})) });
    }
    Ok(out)
  }
  fn save_molecular_property(&self, prop: chem_domain::OwnedMolecularProperty) -> Result<Uuid, DomainError> {
    let mut conn = self.conn()?;
    let row = MolecularPropertyRow { id: prop.id.to_string(),
                                     molecule_inchikey: prop.molecule_inchikey,
                                     property_type: prop.property_type,
                                     value: prop.value.to_string(),
                                     quality: prop.quality,
                                     preferred: prop.preferred,
                                     value_hash: prop.value_hash,
                                     metadata: prop.metadata.to_string() };
    map_db_err(diesel::insert_into(schema::molecular_properties::table).values(&row).execute(&mut conn))?;
    Ok(Uuid::parse_str(&row.id).unwrap())
  }
  fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<chem_domain::OwnedMolecularProperty>, DomainError> {
    let mut conn = self.conn()?;
    let rows =
      molecular_properties_dsl::molecular_properties.filter(molecular_properties_dsl::molecule_inchikey.eq(inchikey))
                                                    .load::<MolecularPropertyRow>(&mut conn)
                                                    .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let mut out = Vec::new();
    for r in rows {
      out.push(chem_domain::OwnedMolecularProperty { id: Uuid::parse_str(&r.id).unwrap(),
                                                     molecule_inchikey: r.molecule_inchikey,
                                                     property_type: r.property_type,
                                                     value:
                                                       serde_json::from_str(&r.value).unwrap_or(serde_json::json!({})),
                                                     quality: r.quality,
                                                     preferred: r.preferred,
                                                     value_hash: r.value_hash,
                                                     metadata:
                                                       serde_json::from_str(&r.metadata).unwrap_or(serde_json::json!({})) });
    }
    Ok(out)
  }
  fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
    let mut conn = self.conn()?;
    use crate::schema::family_members::dsl as fm_dsl;
    // If molecule is referenced in any family_members, do not delete
    let exists = fm_dsl::family_members.filter(fm_dsl::molecule_inchikey.eq(inchikey))
                                       .select(fm_dsl::id)
                                       .first::<String>(&mut conn)
                                       .optional()
                                       .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    if exists.is_some() {
      return Err(DomainError::ValidationError(format!("No se puede eliminar la molecula {}; pertenece a una familia",
                                                      inchikey)));
    }
    use crate::schema::molecules::dsl as mol_dsl;
    map_db_err(diesel::delete(mol_dsl::molecules.filter(mol_dsl::inchikey.eq(inchikey))).execute(&mut conn))?;
    Ok(())
  }
  fn delete_family(&self, id: &Uuid) -> Result<(), DomainError> {
    let mut conn = self.conn()?;
    let id_s = id.to_string();
    use crate::schema::families::dsl as fam_dsl;
    use crate::schema::family_members::dsl as fm_dsl;
    use crate::schema::family_properties::dsl as fp_dsl;
    map_db_err(diesel::delete(fp_dsl::family_properties.filter(fp_dsl::family_id.eq(&id_s))).execute(&mut conn))?;
    map_db_err(diesel::delete(fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))).execute(&mut conn))?;
    map_db_err(diesel::delete(fam_dsl::families.filter(fam_dsl::id.eq(&id_s))).execute(&mut conn))?;
    Ok(())
  }
  fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError> {
    // Perform all DB operations on a single connection to avoid nested pool
    // acquisition (which leads to timeouts with small pools). Use a
    // transaction to keep atomicity.
    let mut conn = self.conn()?;
    use crate::schema::families::dsl as fam_dsl;
    use crate::schema::family_members::dsl as fm_dsl;
    use crate::schema::molecules::dsl as mol_dsl;
    // Use the acquired connection directly (no transaction wrapper) and
    // perform the necessary queries/insertions. This avoids coupling the
    // error types of Diesel and Domain during a transaction closure.
    let id_s = family_id.to_string();
    let fam_row_opt = fam_dsl::families.filter(fam_dsl::id.eq(&id_s))
                                       .first::<FamilyRow>(&mut conn)
                                       .optional()
                                       .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let fam_row = fam_row_opt.ok_or(DomainError::ValidationError("Familia no encontrada".to_string()))?;
    let members = fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))
                                        .load::<FamilyMemberRow>(&mut conn)
                                        .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let mut mols = Vec::new();
    for mem in members {
      if let Ok(Some(mr)) =
        mol_dsl::molecules.filter(mol_dsl::inchikey.eq(&mem.molecule_inchikey)).first::<MoleculeRow>(&mut conn).optional()
      {
        let mol = Molecule::from_parts(&mr.inchikey,
                                       &mr.smiles,
                                       &mr.inchi,
                                       serde_json::from_str(&mr.metadata).unwrap_or(serde_json::json!({})))?;
        mols.push(mol);
      }
    }
    let mut fam_obj = MoleculeFamily::new(mols, serde_json::from_str(&fam_row.provenance).unwrap_or(serde_json::json!({})))?;
    if let Some(n) = fam_row.name.clone() {
      fam_obj = fam_obj.with_name(n)?;
    }
    if let Some(d) = fam_row.description.clone() {
      fam_obj = fam_obj.with_description(d)?;
    }
    let new_fam = fam_obj.add_molecule(molecule.clone())?;
    let mr = MoleculeRow { inchikey: molecule.inchikey().to_string(),
                           smiles: molecule.smiles().to_string(),
                           inchi: molecule.inchi().to_string(),
                           metadata: molecule.metadata().to_string() };
    let _ = diesel::insert_into(mol_dsl::molecules).values(&mr).execute(&mut conn);
    let new_id = new_fam.id().to_string();
    let row = FamilyRow { id: new_id.clone(),
                          name: new_fam.name().map(|s| s.to_string()),
                          description: new_fam.description().map(|s| s.to_string()),
                          family_hash: new_fam.family_hash().to_string(),
                          provenance: new_fam.provenance().to_string(),
                          frozen: new_fam.is_frozen() };
    map_db_err(diesel::insert_into(fam_dsl::families).values(&row).execute(&mut conn))?;
    for m in new_fam.molecules().iter() {
      let fm = FamilyMemberRow { id: Uuid::new_v4().to_string(),
                                 family_id: new_id.clone(),
                                 molecule_inchikey: m.inchikey().to_string() };
      let _ = diesel::insert_into(fm_dsl::family_members).values(&fm).execute(&mut conn);
    }
    Ok(Uuid::parse_str(&new_id).unwrap())
  }
  fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError> {
    // Load family and members using a single connection and create a new
    // family version without the specified molecule; persist in a
    // transaction to avoid nested pool acquisition.
    let mut conn = self.conn()?;
    use crate::schema::families::dsl as fam_dsl;
    use crate::schema::family_members::dsl as fm_dsl;
    use crate::schema::molecules::dsl as mol_dsl;
    let id_s = family_id.to_string();
    let fam_row_opt = fam_dsl::families.filter(fam_dsl::id.eq(&id_s))
                                       .first::<FamilyRow>(&mut conn)
                                       .optional()
                                       .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let fam_row = fam_row_opt.ok_or(DomainError::ValidationError("Familia no encontrada".to_string()))?;
    let members = fm_dsl::family_members.filter(fm_dsl::family_id.eq(&id_s))
                                        .load::<FamilyMemberRow>(&mut conn)
                                        .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
    let mut mols = Vec::new();
    for mem in members {
      if let Ok(Some(mr)) =
        mol_dsl::molecules.filter(mol_dsl::inchikey.eq(&mem.molecule_inchikey)).first::<MoleculeRow>(&mut conn).optional()
      {
        let mol = Molecule::from_parts(&mr.inchikey,
                                       &mr.smiles,
                                       &mr.inchi,
                                       serde_json::from_str(&mr.metadata).unwrap_or(serde_json::json!({})))?;
        mols.push(mol);
      }
    }
    let mut fam_obj = MoleculeFamily::new(mols, serde_json::from_str(&fam_row.provenance).unwrap_or(serde_json::json!({})))?;
    if let Some(n) = fam_row.name.clone() {
      fam_obj = fam_obj.with_name(n)?;
    }
    if let Some(d) = fam_row.description.clone() {
      fam_obj = fam_obj.with_description(d)?;
    }
    let new_fam = fam_obj.remove_molecule(inchikey)?;
    let new_id = new_fam.id().to_string();
    let row = FamilyRow { id: new_id.clone(),
                          name: new_fam.name().map(|s| s.to_string()),
                          description: new_fam.description().map(|s| s.to_string()),
                          family_hash: new_fam.family_hash().to_string(),
                          provenance: new_fam.provenance().to_string(),
                          frozen: new_fam.is_frozen() };
    map_db_err(diesel::insert_into(fam_dsl::families).values(&row).execute(&mut conn))?;
    for m in new_fam.molecules().iter() {
      let fm = FamilyMemberRow { id: Uuid::new_v4().to_string(),
                                 family_id: new_id.clone(),
                                 molecule_inchikey: m.inchikey().to_string() };
      let _ = diesel::insert_into(fm_dsl::family_members).values(&fm).execute(&mut conn);
    }
    Ok(Uuid::parse_str(&new_id).unwrap())
  }
}
/// Crear repo desde las variables de entorno (o default sqlite in-memory en
/// tests)
pub fn new_domain_repo_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  // When compiled with Postgres support prefer CHEM_DB_URL, but allow
  // DATABASE_URL as a fallback (mirrors `flow_persistence::new_from_env`).
  if cfg!(all(feature = "pg", not(test))) {
    let url =
      std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                                DomainError::ExternalError("CHEM_DB_URL / \
                                                                                                            DATABASE_URL \
                                                                                                            not set"
                                                                                                                    .into())
                                                                              })?;
    let l = url.to_lowercase();
    if !(l.starts_with("postgres") || l.starts_with("postgresql://") || url.contains("@")) {
      return Err(DomainError::ExternalError("CHEM_DB_URL / DATABASE_URL does not look like Postgres URL".into()));
    }
    Ok(DieselDomainRepository::new(&url))
  } else {
    let url = std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL"))
                                          .unwrap_or_else(|_| "file:chemdb?mode=memory&cache=shared".into());
    Ok(DieselDomainRepository::new(&url))
  }
}
// Provide a canonical `new_from_env` for the domain repository so callers
// (examples, mains) do not need to decide between sqlite/postgres. This
// mirrors the pattern used by `flow_persistence`.
#[cfg(all(feature = "pg", not(test)))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url =
    std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                              DomainError::ExternalError("CHEM_DB_URL / \
                                                                                                          DATABASE_URL not \
                                                                                                          set"
                                                                                                              .into())
                                                                            })?;
  if !(url.starts_with("postgres") || url.starts_with("postgresql://") || url.contains("@")) {
    return Err(DomainError::ExternalError("chem-persistence: CHEM_DB_URL does not look like Postgres URL".into()));
  }
  Ok(DieselDomainRepository::new(&url))
}
#[cfg(test)]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url = std::env::var("CHEM_DB_URL").unwrap_or_else(|_| "file:memdb1?mode=memory&cache=shared".into());
  let repo = DieselDomainRepository::new(&url);
  Ok(repo)
}
#[cfg(all(not(feature = "pg"), not(test)))]
pub fn new_from_env() -> Result<DieselDomainRepository, DomainError> {
  dotenvy::dotenv().ok();
  let url =
    std::env::var("CHEM_DB_URL").or_else(|_| std::env::var("DATABASE_URL")).map_err(|_| {
                                                                              DomainError::ExternalError("CHEM_DB_URL / \
                                                                                                          DATABASE_URL not \
                                                                                                          set"
                                                                                                              .into())
                                                                            })?;
  let url_l = url.to_lowercase();
  if url_l.starts_with("file:") || url_l.contains("mode=memory") || url_l.contains("sqlite") {
    let repo = DieselDomainRepository::new(&url);
    return Ok(repo);
  }
  Err(DomainError::ExternalError("chem-persistence was compiled without 'pg' feature; enable the 'pg' feature to use \
                                  Postgres in production"
                                                         .into()))
}
// Test helper: construct a DieselDomainRepository backed by explicit SQLite
// connection manager. This bypasses environment parsing and avoids cases
// where the build or features might cause the ConnectionManager to treat
// the string as Postgres connection info.
#[cfg(not(feature = "pg"))]
pub fn new_sqlite_for_test(database_url: &str) -> DieselDomainRepository {
  use diesel::r2d2::ConnectionManager;
  use diesel::sqlite::SqliteConnection;
  let manager = ConnectionManager::<SqliteConnection>::new(database_url);
  let pool = Pool::builder().max_size(4).build(manager).expect("no se pudo crear el pool de conexiones");
  let repo = DieselDomainRepository { pool: Arc::new(pool) };
  if let Ok(mut c) = repo.conn_raw() {
    let _ = diesel::sql_query("PRAGMA journal_mode = WAL;").execute(&mut c);
    let _ = diesel::sql_query("PRAGMA busy_timeout = 5000;").execute(&mut c);
    let _ = c.run_pending_migrations(MIGRATIONS);
  }
  repo
}
