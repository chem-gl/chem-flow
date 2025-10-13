//! Helpers para bases de datos en pruebas
#[cfg(feature = "testing")]
use tempfile::TempDir;
/// Estructura para gestionar una base de datos SQLite temporal
#[cfg(feature = "testing")]
pub struct TempSqliteDb {
  _temp_dir: TempDir,
  pub db_path: std::path::PathBuf,
  pub db_url: String,
}
#[cfg(feature = "testing")]
impl TempSqliteDb {
  /// Crea una nueva base de datos SQLite temporal
  pub fn new() -> Result<Self, std::io::Error> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    Ok(Self { _temp_dir: temp_dir, db_path, db_url })
  }
  /// Obtiene la URL de conexión a la base de datos
  pub fn url(&self) -> &str {
    &self.db_url
  }
}
/// Obtiene una URL de base de datos para pruebas desde variables de entorno
pub fn get_test_db_url() -> String {
  std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string())
}
