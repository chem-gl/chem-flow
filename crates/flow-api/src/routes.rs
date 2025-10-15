//! Configuración de rutas y documentación OpenAPI

use crate::errors::ErrorResponse;
use crate::handlers::family_handlers;
use crate::handlers::molecule_handlers;
use crate::handlers::property_handlers;
use crate::handlers::*;
use crate::models::*;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;
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
  let family_routes =
    Router::new().route("/families", post(family_handlers::create_family)).with_state(Arc::new(state.clone()));

  // Molecule routes (stubs)
  let molecule_routes = Router::new().route("/molecules", post(molecule_handlers::create_molecule))
                                     .route("/molecules", get(molecule_handlers::list_molecules))
                                     .with_state(Arc::new(state.clone()));

  // Property routes (stubs)
  let property_routes = Router::new().route("/properties", post(property_handlers::create_molecular_property))
                                     .with_state(Arc::new(state.clone()));

  // Auth routes (use UserState)
  let user_state = crate::handlers::user_handlers::UserState { user_service: state.user_service.clone() };
  let auth_routes = Router::new().route("/auth/register", post(crate::handlers::user_handlers::register_user))
                                 .route("/auth/login", post(crate::handlers::user_handlers::login))
                                 .with_state(user_state);

  // Team routes (use TeamState)
  let team_state = crate::handlers::team_handlers::TeamState { team_service: state.team_service.clone() };
  let team_routes = Router::new().route("/teams", post(crate::handlers::team_handlers::create_team))
                                 .route("/teams/:id", get(crate::handlers::team_handlers::get_team))
                                 .route("/teams/:id/members", post(crate::handlers::team_handlers::add_member))
                                 .route("/teams/:id/members/:user_id",
                                        delete(crate::handlers::team_handlers::remove_member))
                                 .with_state(team_state);

  // Health check sin estado
  let health_routes = Router::new().route("/health", get(health_check));

  // Router completo combinando todo con Swagger UI
  Router::new().merge(health_routes)
               .nest("/api",
                     api_routes.merge(auth_routes)
                               .merge(team_routes)
                               .merge(family_routes)
                               .merge(molecule_routes)
                               .merge(property_routes))
               .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
}
