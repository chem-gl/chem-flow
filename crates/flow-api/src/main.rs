//! Servidor HTTP principal para Flow-Chem CADMA API
//!
//! Implementa un servidor REST completo con OpenAPI/Swagger y persistencia real

use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod errors;
mod handlers;
mod models;
mod routes;
mod services;

use crate::config::CONFIG;
use crate::handlers::AppState;
use crate::routes::create_router;
use crate::services::CadmaService;

/// Inicializa el sistema de logging
fn init_logging() {
  tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                                                                                             CONFIG.log_level.clone().into()
                                                                                           }))
                                .with(tracing_subscriber::fmt::layer())
                                .init();
}

/// Inicializa los repositorios según la configuración
fn init_repositories(
  )
    -> Result<(Arc<dyn flow::repository::FlowRepository>, Arc<chem_persistence::DieselDomainRepository>)>
{
  tracing::info!("Inicializando repositorios...");
  tracing::info!("DATABASE_URL: {}", CONFIG.database_url);

  // Determinar el tipo de base de datos
  let is_postgres = CONFIG.database_url.starts_with("postgres://") || CONFIG.database_url.starts_with("postgresql://");

  if is_postgres {
    tracing::info!("Usando PostgreSQL");

    // Inicializar repos para PostgreSQL
    let flow_repo =
      chem_persistence::new_flow_from_env().map_err(|e| anyhow::anyhow!("Error inicializando flow repository: {}", e))?;

    let domain_repo =
      chem_persistence::new_domain_from_env().map_err(|e| anyhow::anyhow!("Error inicializando domain repository: {}", e))?;

    Ok((Arc::new(flow_repo), Arc::new(domain_repo)))
  } else {
    tracing::info!("Usando SQLite");

    // Para SQLite (por defecto en chem-persistence)
    let flow_repo =
      chem_persistence::new_flow_from_env().map_err(|e| anyhow::anyhow!("Error inicializando flow repository: {}", e))?;

    let domain_repo =
      chem_persistence::new_domain_from_env().map_err(|e| anyhow::anyhow!("Error inicializando domain repository: {}", e))?;

    Ok((Arc::new(flow_repo), Arc::new(domain_repo)))
  }
}

/// Punto de entrada principal
#[tokio::main]
async fn main() -> Result<()> {
  // Cargar configuración
  let config = &*CONFIG;

  // Inicializar logging
  init_logging();

  tracing::info!("🚀 Iniciando Flow-Chem CADMA API...");
  tracing::info!("Modo: {}", if config.is_dev { "Desarrollo" } else { "Producción" });

  // Inicializar repositorios
  let (flow_repo, domain_repo) = init_repositories()?;

  tracing::info!("✅ Repositorios inicializados correctamente");

  // Crear servicio CADMA
  let cadma_service = Arc::new(CadmaService::new(flow_repo, domain_repo));

  // Crear estado de la aplicación
  let app_state = AppState { cadma_service };

  // Crear router con todas las rutas
  let app = create_router(app_state).layer(TraceLayer::new_for_http());

  // Configurar listener
  let addr = config.server_address();
  let listener = TcpListener::bind(&addr).await?;

  tracing::info!("✅ Servidor escuchando en http://{}", addr);
  tracing::info!("📚 Documentación Swagger disponible en http://{}/docs", addr);
  tracing::info!("📄 OpenAPI JSON disponible en http://{}/api-doc/openapi.json", addr);

  // Iniciar servidor
  axum::serve(listener, app).await?;

  Ok(())
}
