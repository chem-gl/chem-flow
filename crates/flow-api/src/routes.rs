//! Configuración de rutas y documentación OpenAPI

use crate::errors::ErrorResponse;
use crate::handlers::*;
use crate::handlers::family_handlers;
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
    crate::handlers::user_handlers::register_user,
    crate::handlers::user_handlers::login,
    crate::handlers::team_handlers::create_team,
    crate::handlers::team_handlers::get_team,
    crate::handlers::team_handlers::add_member,
    crate::handlers::team_handlers::remove_member,
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
      RegisterUserRequest,
      LoginRequest,
      LoginResponse,
      UserResponse,
      CreateTeamRequest,
      TeamResponse,
      TeamMemberRequest,
    )
  ),
  tags(
    (name = "CADMA Workflow", description = "Endpoints para gestionar ejecuciones del workflow CADMA"),
    (name = "Health", description = "Endpoints de salud y monitoreo"),
    (name = "Auth", description = "Registro e inicio de sesión"),
    (name = "Teams", description = "Gestión de equipos")
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
  let api_routes = Router::new().route("/flows/cadma/start", post(start_cadma))
                                .route("/flows/cadma", get(list_executions))
                                .route("/flows/cadma/:id", get(get_cadma_status).delete(cancel_execution))
                                .route("/flows/cadma/:id/step", post(execute_step))
                                .with_state(state.clone());

    // Family routes (create only for now)
    let family_routes = Router::new().route("/families", post(family_handlers::create_family)).with_state(Arc::new(state.clone()));

  // Auth and team routes are currently disabled until the user/team services
  // are fully implemented and wired into AppState. Keep only CADMA and
  // health routes for the integration tests.
  let auth_routes = Router::new();
  let team_routes = Router::new();

  // Health check sin estado
  let health_routes = Router::new().route("/health", get(health_check));

  // Router completo combinando todo con Swagger UI
  Router::new().merge(health_routes)
               .nest("/api", api_routes.merge(auth_routes).merge(team_routes))
                 .merge(family_routes)
               .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
}
