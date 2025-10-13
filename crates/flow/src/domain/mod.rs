//! Módulo de dominio - Core del sistema de flujos
//!
//! Este módulo contiene la lógica de negocio central del sistema de flujos,
//! implementando el algoritmo de árbol dirigido acíclico (DAG) con las
//! siguientes características:
//!
//! - **Sin Ciclos**: Estructura de árbol estricta que previene bucles
//! - **Sin Merges**: No permite fusión de ramas divergentes
//! - **Sin Duplicaciones**: Verificación global de unicidad de contenido
//! - **Eliminación Recursiva**: Borrado automático de subramas dependientes
//!
//! ## Estructura
//!
//! - `entities`: Objetos con identidad y ciclo de vida (FlowNode, FlowBranch,
//!   FlowTree)
//! - `value_objects`: Objetos inmutables definidos por su valor (FlowData, IDs,
//!   Commands)
//! - `services`: Lógica de negocio que opera sobre múltiples entidades

pub mod entities;
pub mod services;
pub mod value_objects;

// Re-exports públicos para facilitar el uso
pub use entities::*;
pub use services::*;
pub use value_objects::*;

// Backwards-compat alias: older code used FlowMeta as name for FlowMetadata.
pub use value_objects::FlowMetadata as FlowMeta;
