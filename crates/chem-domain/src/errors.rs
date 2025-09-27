// error.rs
use chem_providers::EngineError;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum DomainError {
  #[error("Error de validación: {0}")]
  ValidationError(String),
  #[error("Error externo: {0}")]
  ExternalError(String),
  #[error("Error de serialización: {0}")]
  SerializationError(String),
}

impl From<EngineError> for DomainError {
  fn from(e: EngineError) -> Self {
    Self::ExternalError(e.to_string())
  }
}

impl From<serde_json::Error> for DomainError {
  fn from(e: serde_json::Error) -> Self {
    Self::SerializationError(e.to_string())
  }
}
