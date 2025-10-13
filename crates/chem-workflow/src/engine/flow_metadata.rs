//! Metadatos del flujo de trabajo.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Estructura tipada para los metadatos del flujo.
/// Se almacena bajo la clave `flow_metadata` en el repositorio.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FlowMetadata {
  /// Paso actual del flujo.
  #[serde(default)]
  pub current_step: u32,
  /// Estado actual del flujo.
  #[serde(default)]
  pub status: String,
  /// Referencias a entidades del dominio.
  #[serde(default)]
  pub domain_refs: Vec<Uuid>,
}

impl FlowMetadata {
  /// Crea una nueva instancia de metadatos.
  pub fn new(current_step: u32, status: impl Into<String>, domain_refs: Vec<Uuid>) -> Self {
    Self { current_step, status: status.into(), domain_refs }
  }
}
