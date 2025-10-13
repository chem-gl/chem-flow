//! Capa de aplicación - Casos de uso y servicios de aplicación
//!
//! Esta capa orquesta las operaciones del dominio y coordina con los puertos
//! para implementar los casos de uso del sistema.
//!
//! ## Estructura
//!
//! - `use_cases`: Casos de uso específicos que encapsulan lógica de aplicación
//! - `services`: Implementaciones de los puertos de entrada que orquestan casos
//!   de uso
//!
//! ## Responsabilidades
//!
//! - Orquestar operaciones del dominio
//! - Coordinar con repositorios y servicios externos
//! - Implementar transacciones y control de flujo
//! - Publicar eventos de dominio
//! - Validar precondiciones de negocio

pub mod services;
pub mod use_cases;

// Re-exports públicos
pub use services::*;
pub use use_cases::*;
