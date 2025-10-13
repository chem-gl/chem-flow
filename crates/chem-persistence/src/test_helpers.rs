#![allow(dead_code)]
#[cfg(feature = "sqlite")]
use crate::db::{init_sqlite_pool_from_path, run_migrations_on_sqlite_pool, SqlitePool};
#[cfg(feature = "sqlite")]
use anyhow::{Context, Result};
#[cfg(feature = "sqlite")]
use std::path::{Path, PathBuf};
#[cfg(feature = "sqlite")]
use uuid::Uuid;
#[cfg(feature = "sqlite")]
#[derive(Debug)]
pub struct TestSqliteDb {
  pub pool: SqlitePool,
  pub path: PathBuf,
  keep_file: bool,
}
#[cfg(feature = "sqlite")]
impl TestSqliteDb {
  pub fn path(&self) -> &Path {
    &self.path
  }
  pub fn keep(mut self) -> Self {
    self.keep_file = true;
    self
  }
}
#[cfg(feature = "sqlite")]
impl Drop for TestSqliteDb {
  fn drop(&mut self) {
    if !self.keep_file {
      let _ = std::fs::remove_file(&self.path);
    }
  }
}
#[cfg(feature = "sqlite")]
pub fn create_temp_sqlite_db() -> Result<TestSqliteDb> {
  create_temp_sqlite_db_with_options(false)
}
#[cfg(feature = "sqlite")]
pub fn create_temp_sqlite_db_with_options(keep_file: bool) -> Result<TestSqliteDb> {
  let filename = format!("chemflow-test-{}.db", Uuid::new_v4());
  let path = std::env::temp_dir().join(filename);
  create_sqlite_db_at(path, keep_file)
}
#[cfg(feature = "sqlite")]
pub fn create_sqlite_db_at<P: AsRef<Path>>(path: P, keep_file: bool) -> Result<TestSqliteDb> {
  let path_ref = path.as_ref();
  let pool = init_sqlite_pool_from_path(path_ref).context("initializing sqlite pool")?;
  run_migrations_on_sqlite_pool(&pool).context("running sqlite migrations")?;
  Ok(TestSqliteDb { pool, path: path_ref.to_path_buf(), keep_file })
}
