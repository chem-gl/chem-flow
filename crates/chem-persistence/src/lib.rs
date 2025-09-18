//! Implementación mínima de persistencia para el trait `FlowRepository`.
//! Este archivo expone el módulo `schema` y reexporta el repositorio Diesel
//! que implementa los traits de persistencia del dominio. La implementación
//! detallada está en `domain_persistence.rs`.

mod domain_persistence;
mod flow_persistence;
pub mod schema;

#[cfg(not(feature = "pg"))]
pub use domain_persistence::new_sqlite_for_test;
pub use domain_persistence::{new_domain_repo_from_env, new_from_env as new_domain_from_env, DieselDomainRepository};
pub use flow_persistence::{new_from_env, DieselFlowRepository};
