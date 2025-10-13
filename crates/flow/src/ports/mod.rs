//! Puertos - Interfaces de la arquitectura hexagonal
//!
//! Define las abstracciones que separan el dominio de los detalles de
//! implementación. Los puertos actúan como contratos que deben ser cumplidos
//! por los adaptadores.
//!
//! ## Estructura
//!
//! - `inbound`: Interfaces que expone el dominio hacia el exterior (casos de
//!   uso)
//! - `outbound`: Interfaces que requiere el dominio del exterior (persistencia,
//!   etc.)
//!
//! ## Principios Aplicados
//!
//! - **Dependency Inversion**: El dominio define las interfaces que necesita
//! - **Interface Segregation**: Interfaces específicas y cohesivas
//! - **Single Responsibility**: Cada puerto tiene una responsabilidad clara

pub mod inbound;
pub mod outbound;

// Re-exports públicos
pub use inbound::*;
pub use outbound::*;
