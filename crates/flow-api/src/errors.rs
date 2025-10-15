//! Módulo de errores para la API RESTful
//!
//! Define errores comunes y su mapeo a respuestas HTTP

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

/// Error principal de la API
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
  #[error("Entidad no encontrada: {0}")]
  NotFound(String),

  #[error("Solicitud inválida: {0}")]
  BadRequest(String),

  #[error("Conflicto: {0}")]
  Conflict(String),

  #[error("Error interno del servidor: {0}")]
  InternalError(String),

  #[error("Error de validación: {0}")]
  ValidationError(String),

  #[error("Error de workflow: {0}")]
  WorkflowError(String),

  #[error("Error de persistencia: {0}")]
  PersistenceError(String),

  #[error("Error de dominio: {0}")]
  DomainError(String),
  #[error("No autorizado: {0}")]
  Unauthorized(String),
}

/// Cuerpo de respuesta de error estandarizado
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorResponse {
  /// Código de error HTTP
  pub status: u16,

  /// Mensaje de error descriptivo
  pub message: String,

  /// Timestamp del error
  pub timestamp: String,
}

impl ErrorResponse {
  pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
    Self { status: status.as_u16(), message: message.into(), timestamp: chrono::Utc::now().to_rfc3339() }
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let (status, message) = match self {
      ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
      ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
      ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
      ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
      ApiError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
      ApiError::WorkflowError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
      ApiError::PersistenceError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
      ApiError::DomainError(msg) => (StatusCode::BAD_REQUEST, msg),
      ApiError::Unauthorized(msg) => (StatusCode::FORBIDDEN, msg),
    };

    let error_response = ErrorResponse::new(status, message);

    (status, Json(error_response)).into_response()
  }
}

// Conversiones desde errores del workspace
impl From<chem_workflow::WorkflowError> for ApiError {
  fn from(err: chem_workflow::WorkflowError) -> Self {
    ApiError::WorkflowError(err.to_string())
  }
}

impl From<chem_domain::DomainError> for ApiError {
  fn from(err: chem_domain::DomainError) -> Self {
    ApiError::DomainError(err.to_string())
  }
}

impl From<flow::errors::FlowError> for ApiError {
  fn from(err: flow::errors::FlowError) -> Self {
    ApiError::PersistenceError(err.to_string())
  }
}

impl From<anyhow::Error> for ApiError {
  fn from(err: anyhow::Error) -> Self {
    ApiError::InternalError(err.to_string())
  }
}

impl From<serde_json::Error> for ApiError {
  fn from(err: serde_json::Error) -> Self {
    ApiError::BadRequest(format!("Error de serialización JSON: {}", err))
  }
}
