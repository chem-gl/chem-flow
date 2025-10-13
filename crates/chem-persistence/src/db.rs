use crate::migrations::MIGRATIONS;
use anyhow::{anyhow, Context, Result};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::Connection;
#[cfg(feature = "postgres")]
pub type PostgresPool = Pool<ConnectionManager<diesel::pg::PgConnection>>;
#[cfg(feature = "sqlite")]
pub type SqlitePool = Pool<ConnectionManager<diesel::sqlite::SqliteConnection>>;
#[cfg(feature = "postgres")]
pub fn init_postgres_pool_from_url(database_url: &str) -> Result<PostgresPool> {
  let manager = ConnectionManager::<diesel::pg::PgConnection>::new(database_url);
  let pool = Pool::builder().build(manager).context("failed to build Postgres connection pool")?;
  Ok(pool)
}
#[cfg(feature = "sqlite")]
pub fn init_sqlite_pool_from_path(path: &std::path::Path) -> Result<SqlitePool> {
  let db_path = path.to_str().context("sqlite path contains invalid UTF-8")?.to_string();
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent).context("failed to create sqlite parent directory")?;
  }
  let manager = ConnectionManager::<diesel::sqlite::SqliteConnection>::new(db_path);
  let pool = Pool::builder().max_size(4).build(manager).context("failed to build sqlite connection pool")?;
  configure_sqlite_pool(&pool)?;
  Ok(pool)
}
#[cfg(feature = "sqlite")]
fn configure_sqlite_pool(pool: &SqlitePool) -> Result<()> {
  use diesel::RunQueryDsl;
  let mut conn = pool.get().context("failed to fetch sqlite connection")?;
  diesel::sql_query("PRAGMA foreign_keys = ON;").execute(&mut conn).context("failed to enable foreign_keys")?;
  diesel::sql_query("PRAGMA journal_mode = WAL;").execute(&mut conn).context("failed to set journal_mode")?;
  diesel::sql_query("PRAGMA synchronous = NORMAL;").execute(&mut conn).context("failed to set synchronous")?;
  Ok(())
}
pub fn run_migrations_on_connection<C>(conn: &mut C) -> Result<()>
  where C: Connection + diesel_migrations::MigrationHarness<C::Backend>
{
  let applied = conn.run_pending_migrations(MIGRATIONS).map_err(|e| anyhow!("running embedded migrations: {}", e))?;
  if !applied.is_empty() {
    log::info!("chem-persistence: applied {} migrations", applied.len());
  }
  Ok(())
}
#[cfg(feature = "postgres")]
pub fn run_migrations_on_pg_pool(pool: &PostgresPool) -> Result<()> {
  let mut conn = pool.get().context("failed to fetch connection for migrations")?;
  run_migrations_on_connection(&mut conn)
}
#[cfg(feature = "sqlite")]
pub fn run_migrations_on_sqlite_pool(pool: &SqlitePool) -> Result<()> {
  let mut conn = pool.get().context("failed to fetch connection for migrations")?;
  run_migrations_on_connection(&mut conn)
}
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
pub use run_migrations_on_pg_pool as run_migrations_on_pool;
#[cfg(all(feature = "postgres", feature = "sqlite"))]
pub use run_migrations_on_pg_pool as run_migrations_on_pool;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use run_migrations_on_sqlite_pool as run_migrations_on_pool;
