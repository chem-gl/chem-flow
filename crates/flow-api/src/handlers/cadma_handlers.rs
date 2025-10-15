//! Handlers HTTP para los endpoints de CADMA
//!
//! Implementa todos los endpoints REST con documentación OpenAPI

use crate::errors::{ApiError, ErrorResponse};
use crate::models::*;
use crate::services::{CadmaService, FamilyService};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;
use uuid::Uuid;

/// Estado compartido de la aplicación
#[derive(Clone)]
pub struct AppState {
  pub cadma_service: Arc<CadmaService>,
  pub family_service: Arc<FamilyService>,
  pub molecule_service: Arc<crate::services::MoleculeService>,
  pub property_service: Arc<crate::services::PropertyService>,
  pub user_service: Arc<crate::services::UserService>,
  pub team_service: Arc<crate::services::TeamService>,
}

// ============================================================================
// Handlers principales
// ============================================================================

/// Inicia una nueva ejecución de CADMA
///
/// Crea un nuevo flow persistido y devuelve su ID único
#[utoipa::path(
  post,
  path = "/api/flows/cadma/start",
  request_body = StartCadmaRequest,
  responses(
    (status = 201, description = "Ejecución creada exitosamente", body = StartCadmaResponse),
    (status = 400, description = "Solicitud inválida", body = ErrorResponse),
    (status = 500, description = "Error interno del servidor", body = ErrorResponse)
  ),
  tag = "CADMA Workflow"
)]
pub async fn start_cadma(State(state): State<AppState>,
                         Json(req): Json<StartCadmaRequest>)
                         -> Result<(StatusCode, Json<StartCadmaResponse>), ApiError> {
  tracing::info!("Iniciando nueva ejecución CADMA: {:?}", req.name);

  let response = state.cadma_service.start_execution(req)?;

  tracing::info!("Ejecución creada con ID: {}", response.execution_id);

  Ok((StatusCode::CREATED, Json(response)))
}

/// Obtiene el estado de una ejecución
///
/// Devuelve información completa sobre el estado actual y pasos completados
#[utoipa::path(
  get,
  path = "/api/flows/cadma/{id}",
  params(
    ("id" = Uuid, Path, description = "ID único de la ejecución")
  ),
  responses(
    (status = 200, description = "Estado obtenido exitosamente", body = CadmaExecutionStatus),
    (status = 404, description = "Ejecución no encontrada", body = ErrorResponse),
    (status = 500, description = "Error interno del servidor", body = ErrorResponse)
  ),
  tag = "CADMA Workflow"
)]
pub async fn get_cadma_status(State(state): State<AppState>,
                              Path(id): Path<Uuid>)
                              -> Result<Json<CadmaExecutionStatus>, ApiError> {
  tracing::debug!("Consultando estado de ejecución: {}", id);

  let status = state.cadma_service.get_execution_status(id)?;

  Ok(Json(status))
}

/// Ejecuta un paso específico del workflow
///
/// Permite ejecutar cualquier paso del workflow CADMA (0-5) enviando el payload
/// apropiado
#[utoipa::path(
  post,
  path = "/api/flows/cadma/{id}/step",
  params(
    ("id" = Uuid, Path, description = "ID único de la ejecución")
  ),
  request_body = ExecuteStepRequest,
  responses(
    (status = 200, description = "Paso ejecutado exitosamente", body = ExecuteStepResponse),
    (status = 400, description = "Solicitud inválida o paso no válido", body = ErrorResponse),
    (status = 404, description = "Ejecución no encontrada", body = ErrorResponse),
    (status = 500, description = "Error interno del servidor", body = ErrorResponse)
  ),
  tag = "CADMA Workflow"
)]
pub async fn execute_step(State(state): State<AppState>,
                          Path(id): Path<Uuid>,
                          Json(req): Json<ExecuteStepRequest>)
                          -> Result<Json<ExecuteStepResponse>, ApiError> {
  tracing::info!("Ejecutando paso {} para ejecución {}", req.step_index, id);

  let response = state.cadma_service.execute_step(id, req.step_index, req.payload)?;

  tracing::info!("Paso {} completado para ejecución {}", req.step_index, id);

  Ok(Json(response))
}

/// Cancela una ejecución en curso
///
/// Elimina o marca como cancelada una ejecución específica
#[utoipa::path(
  delete,
  path = "/api/flows/cadma/{id}",
  params(
    ("id" = Uuid, Path, description = "ID único de la ejecución")
  ),
  responses(
    (status = 200, description = "Ejecución cancelada exitosamente", body = CancelExecutionResponse),
    (status = 404, description = "Ejecución no encontrada", body = ErrorResponse),
    (status = 500, description = "Error interno del servidor", body = ErrorResponse)
  ),
  tag = "CADMA Workflow"
)]
pub async fn cancel_execution(State(state): State<AppState>,
                              Path(id): Path<Uuid>)
                              -> Result<Json<CancelExecutionResponse>, ApiError> {
  tracing::info!("Cancelando ejecución: {}", id);

  let response = state.cadma_service.cancel_execution(id)?;

  tracing::info!("Ejecución {} cancelada", id);

  Ok(Json(response))
}

/// Lista todas las ejecuciones
///
/// Devuelve un listado de todas las ejecuciones activas e históricas
#[utoipa::path(
  get,
  path = "/api/flows/cadma",
  responses(
    (status = 200, description = "Lista obtenida exitosamente", body = ListExecutionsResponse),
    (status = 500, description = "Error interno del servidor", body = ErrorResponse)
  ),
  tag = "CADMA Workflow"
)]
pub async fn list_executions(State(state): State<AppState>) -> Result<Json<ListExecutionsResponse>, ApiError> {
  tracing::debug!("Listando todas las ejecuciones");

  let response = state.cadma_service.list_executions()?;

  Ok(Json(response))
}

// ============================================================================
// Health check
// ============================================================================

/// Health check endpoint
///
/// Verifica que la API está operativa
#[utoipa::path(
  get,
  path = "/health",
  responses(
    (status = 200, description = "API operativa", body = SuccessResponse)
  ),
  tag = "Health"
)]
pub async fn health_check() -> Json<SuccessResponse> {
  Json(SuccessResponse { message: "API CADMA operativa".to_string() })
}
