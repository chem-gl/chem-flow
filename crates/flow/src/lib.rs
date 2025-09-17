//! Este módulo raíz expone los componentes públicos del crate `flow`.
//!
//! Propósito: proporcionar tipos y traits para persistir `FlowData`, gestionar
//! snapshots y crear ramas. La lógica de ejecución de pasos se delega a un
//! motor externo que consume los registros persistidos.
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
