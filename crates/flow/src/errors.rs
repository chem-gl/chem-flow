// Archivo: errors.rs
// Propósito: definir los errores del dominio y el alias Result<T> usado por
// las APIs del crate. Los comentarios y variantes están en español.
use thiserror::Error;
/// Errores comunes del dominio de flujos.
///
/// - `NotFound`: entidad no encontrada.
/// - `Conflict`: conflicto de concurrencia o versión.
/// - `Storage`: error al acceder al almacenamiento externo.
/// - `Other`: cualquier otro error.
#[derive(Error, Debug)]
pub enum FlowError {
  /// Entidad no encontrada (por ejemplo, flow o snapshot).
  #[error("No encontrado: {0}")]
  NotFound(String),
  /// Conflicto optimista (version/expected mismatch).
  #[error("Conflicto: {0}")]
  Conflict(String),
  /// Error genérico de almacenamiento (BD, S3, etc.).
  #[error("Error de almacenamiento: {0}")]
  Storage(String),
  /// Otro tipo de error.
  #[error("Otro: {0}")]
  Other(String),
}
/// Alias de resultado usado por las APIs del crate.
pub type Result<T> = std::result::Result<T, FlowError>;
