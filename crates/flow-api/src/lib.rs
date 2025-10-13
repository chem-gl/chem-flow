//! Flow-Chem CADMA API
//!
//! API RESTful para ejecutar workflows químicos CADMA con persistencia real

pub mod config;
pub mod errors;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

// Re-exports para facilitar el uso
pub use config::AppConfig;
pub use errors::ApiError;
pub use handlers::AppState;
pub use routes::create_router;
pub use services::CadmaService;
