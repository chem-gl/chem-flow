//! Crate `flow` — tipos y traits para persistencia basada en registros
//!
//! Este crate define los tipos de dominio (por ejemplo `FlowData`, `FlowMeta`),
//! el contrato de persistencia `FlowRepository` y una implementación en memoria
//! útil para pruebas (`InMemoryFlowRepository`). También expone un motor
//! auxiliar `FlowEngine` con helpers ergonómicos para crear flujos, añadir
//! pasos, crear ramas y gestionar snapshots.
//!
//! Diseño resumido:
//! - Persistencia por registros: cada `FlowData` es autocontenido y permite
//!   reconstruir estado mediante snapshot + replay.
//! - Idempotencia: se admite `command_id` en `FlowData` para evitar duplicados.
//! - Locking optimista: operaciones que modifican un flujo usan un
//!   `expected_version` para detectar conflictos (`PersistResult::Conflict`).
//!
//! Ejemplo rápido:
//! ```rust
//! use flow::stubs::InMemoryFlowRepository;
//! use flow::engine::FlowEngineConfig;
//! use std::sync::Arc;
//! let repo = Arc::new(InMemoryFlowRepository::new());
//! let engine = flow::FlowEngine::new(repo, FlowEngineConfig {});
//! ```
pub mod domain;
pub mod engine;
pub mod errors;
pub mod repository;
pub mod stubs;

pub use domain::*;
pub use engine::*;
pub use errors::*;
pub use repository::*;
pub use stubs::*;
