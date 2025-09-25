// cadma_flow.rs
//
// Implementación concreta de un engine de ejemplo (CadmaFlow).
// Esta implementación es intencionalmente mínima y delega la mayor
// parte de la lógica común al trait `ChemicalFlowEngine` mediante la
// macro `impl_chemical_flow!`.
use crate::{flows::cadma_flow::steps::FamilyReferenceStep1, WorkflowType};
use chem_domain::DomainRepository;
use flow::repository::FlowRepository;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadmaState {
  pub current_step: u32,
  pub domain_refs: Vec<Uuid>,
  pub metadata: JsonValue,
  pub status: String,
}
impl Default for CadmaState {
  fn default() -> Self {
    CadmaState { current_step: 0,
                 domain_refs: Vec::new(),
                 metadata: JsonValue::Object(serde_json::Map::new()),
                 status: "not_started".to_string() }
  }
}

#[derive(Clone)]
pub struct CadmaFlow {
  pub id: Uuid,
  pub state: CadmaState,
  pub flow_repo: Arc<dyn FlowRepository>,
  pub domain_repo: Arc<dyn DomainRepository>,
}
crate::impl_chemical_flow!(CadmaFlow, CadmaState, WorkflowType::Cadma, { 0 => FamilyReferenceStep1 });
