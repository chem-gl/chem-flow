
//! Implementación mínima de persistencia para el trait `FlowRepository`.
//! Este archivo expone el módulo `schema` y reexporta el repositorio Diesel
//! que implementa los traits de persistencia del dominio. La implementación
//! detallada está en `domain_persistence.rs`.

pub mod schema;
mod flow_persistence;

pub use flow_persistence::{DieselFlowRepository, new_from_env};
