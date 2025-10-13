//! Configuración de rutas y documentación OpenAPI

use crate::errors::ErrorResponse;
use crate::handlers::*;
use crate::models::*;
use axum::routing::{get, post};
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Definición de la documentación OpenAPI
#[derive(OpenApi)]
#[openapi(
  paths(
    crate::handlers::cadma_handlers::start_cadma,
    crate::handlers::cadma_handlers::get_cadma_status,
    crate::handlers::cadma_handlers::execute_step,
    crate::handlers::cadma_handlers::cancel_execution,
    crate::handlers::cadma_handlers::list_executions,
    crate::handlers::cadma_handlers::health_check,
  ),
  components(
    schemas(
      StartCadmaRequest,
      ExecuteStepRequest,
      Step1InputDto,
      Step2InputDto,
      Step3InputDto,
      Step4InputDto,
      Step5InputDto,
      Step6InputDto,
      
      StartCadmaResponse,
      CadmaExecutionStatus,
      ExecuteStepResponse,
      CancelExecutionResponse,
      ListExecutionsResponse,
      ExecutionSummary,
      StepInfo,
      SuccessResponse,
      ErrorResponse,
    )
  ),
  tags(
    (name = "CADMA Workflow", description = "Endpoints para gestionar ejecuciones del workflow CADMA"),
    (name = "Health", description = "Endpoints de salud y monitoreo")
  ),
  info(
    title = "Flow-Chem CADMA API",
    version = "0.1.0",
    description = "API RESTful para ejecutar workflows químicos CADMA con persistencia en PostgreSQL/SQLite",
    contact(
      name = "Flow-Chem Team",
      email = "info@flow-chem.org"
    ),
    license(
      name = "MIT"
    )
  )
)]
pub struct ApiDoc;

/// Crea el router principal con todas las rutas
pub fn create_router(state: AppState) -> Router {
  // Rutas de API principal con estado
  let api_routes = Router::new()
    .route("/flows/cadma/start", post(start_cadma))
    .route("/flows/cadma", get(list_executions))
    .route("/flows/cadma/:id", get(get_cadma_status).delete(cancel_execution))
    .route("/flows/cadma/:id/step", post(execute_step))
    .with_state(state.clone());

  // Health check sin estado
  let health_routes = Router::new().route("/health", get(health_check));

  // Router completo combinando todo con Swagger UI
  Router::new()
    .merge(health_routes)
    .nest("/api", api_routes)
    .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
}
