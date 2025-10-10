//! Implementación mínima de persistencia para el trait `FlowRepository`.
//! Este archivo expone el módulo `schema` y reexporta el repositorio Diesel
//! que implementa los traits de persistencia del dominio. La implementación
//! detallada está en `domain_persistence.rs`.
mod db;
mod domain_persistence;
mod flow_persistence;
mod migrations;
pub mod schema;
#[cfg(any(test, feature = "sqlite"))]
pub mod test_helpers;
#[cfg(feature = "postgres")]
pub use db::{init_postgres_pool_from_url, PostgresPool};
#[cfg(feature = "sqlite")]
pub use db::{init_sqlite_pool_from_path, SqlitePool};
pub use db::{run_migrations_on_connection, run_migrations_on_pool};
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub use domain_persistence::new_sqlite_for_test;
pub use domain_persistence::{new_domain_repo_from_env, new_from_env as new_domain_from_env, DieselDomainRepository};
pub use flow_persistence::{new_from_env as new_flow_from_env, DieselFlowRepository};
