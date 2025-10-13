//! Adaptadores - Implementaciones concretas de los puertos
//!
//! Los adaptadores implementan las interfaces definidas en los puertos,
//! conectando el dominio con tecnologías específicas de infraestructura.
//!
//! ## Tipos de Adaptadores
//!
//! - **Persistencia**: Repositorios para bases de datos, sistemas de archivos,
//!   etc.
//! - **Comunicación**: Clientes HTTP, message brokers, etc.
//! - **Infraestructura**: Servicios de logging, métricas, etc.
//!
//! ## Implementaciones Actuales
//!
//! - `memory_repository`: Implementación en memoria para testing y desarrollo

pub mod memory_repository;

// Re-exports públicos para mantener compatibilidad
pub use memory_repository::InMemoryFlowRepository;
// Note: stubs removed; InMemoryFlowRepository is exported above from memory
// adapter
