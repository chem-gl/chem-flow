use thiserror::Error;

// Errores comunes del motor de workflow.
//
// Este enum centraliza los errores que pueden ocurrir durante la
// ejecucion del flujo: errores de persistencia (`FlowError`), errores
// del dominio (`DomainError`), validaciones y errores de serializacion.
#[derive(Error, Debug)]
pub enum WorkflowError {
  /// Errores originados por la capa de persistencia/flow crate.
  #[error("Error de flujo: {0}")]
  Flow(#[from] flow::errors::FlowError),

  /// Errores originados por operaciones del dominio quimico.
  #[error("Error de dominio: {0}")]
  Domain(#[from] chem_domain::DomainError),

  /// Errores de persistencia de alto nivel (mensajes simples).
  #[error("Error de persistencia: {0}")]
  Persistence(String),

  /// Errores de serializacion/deserializacion JSON.
  #[error("Error de serializacion: {0}")]
  Serialization(#[from] serde_json::Error),

  /// Errores de validacion local del workflow (por ejemplo indices
  /// de pasos invalidos).
  #[error("Error de validacion: {0}")]
  Validation(String),

  /// Error generico: captura otros tipos de errores no tipados.
  #[error("Otro error: {0}")]
  Other(String),
}
