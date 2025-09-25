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
//!
//! Rehidratación y puntos de guardado (documentación en español):
//!
//! - Convención de guardado de pasos: los resultados de cada paso se persisten
//!   como un registro `FlowData` con key `step_state:{step_name}`. El campo
//!   `payload` de ese `FlowData` contiene el DTO serializado (por ejemplo
//!   `Step2Payload`) y `metadata` contiene status/params y referencias a
//!   objetos de dominio. Esta convención permite que otros pasos o herramientas
//!   busquen rápidamente el último resultado de un paso concreto.
//!
//! - Snapshots: para acelerar la reconstrucción del estado completo del motor,
//!   se pueden guardar snapshots (blob) que contienen una representación
//!   serializada del estado (por ejemplo `CadmaState`). El repositorio expone
//!   `load_latest_snapshot` y `load_snapshot` para recuperar el snapshot más
//!   reciente y su contenido.
//!
//! - Rehidratación (pasos prácticos): un proceso de rehidratación habitual
//!   consta de:
//!   1) Llamar a `FlowRepository::load_latest_snapshot(flow_id)` para obtener
//!      metadata del último snapshot (si existe).
//!   2) Si hay snapshot, llamar a `FlowRepository::load_snapshot(snapshot_id)`
//!      para recuperar los bytes del snapshot y deserializarlos al estado del
//!      engine.
//!   3) Leer los `FlowData` relevantes con `FlowRepository::read_data(flow_id,
//!      from_cursor)` y aplicar (replay) esos eventos sobre el estado
//!      reconstruido si el snapshot no estaba completo hasta el cursor deseado.
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
