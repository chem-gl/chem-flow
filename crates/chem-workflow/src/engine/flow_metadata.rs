use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Esquema tipado para los metadatos del flujo.
/// Se almacena bajo la clave `flow_metadata` en el repositorio.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FlowMetadata {
  #[serde(default)]
  pub current_step: u32,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub domain_refs: Vec<Uuid>,
}

impl FlowMetadata {
  pub fn new(current_step: u32, status: impl Into<String>, domain_refs: Vec<Uuid>) -> Self {
    Self { current_step, status: status.into(), domain_refs }
  }
}
