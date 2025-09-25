// context.rs
//
// Provee `StepContext`, un helper ligero que facilita a los pasos
// acceder a la persistencia (FlowRepository) y al DomainRepository.
// Incluye utilidades para leer el último payload tipado y para
// persistir resultados tipados de pasos.
use crate::errors::WorkflowError;
use crate::step::StepInfo;
use chem_domain::DomainRepository;
use flow::repository::FlowRepository;
use flow::PersistResult;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use uuid::Uuid;
pub struct StepContext {
  pub flow_id: Uuid,
  pub flow_repo: Arc<dyn FlowRepository>,
  pub domain_repo: Arc<dyn DomainRepository>,
}
impl StepContext {
  /// Crea un nuevo contexto para el flow indicado.
  pub fn new(flow_id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self {
    Self { flow_id, flow_repo, domain_repo }
  }
  /// Obtiene el último output tipado del flujo
  pub fn get_typed_output_by_type<T>(&self) -> Result<Option<T>, WorkflowError>
    where T: DeserializeOwned
  {
    let data = self.flow_repo.read_data(&self.flow_id, 0)?;
    for fd in data.iter().rev() {
      if let Ok(v) = serde_json::from_value::<T>(fd.payload.clone()) {
        return Ok(Some(v));
      }
    }
    Ok(None)
  }
  /// Persiste un resultado tipado de paso
  pub fn save_typed_result(&self,
                           step_name: &str,
                           info: StepInfo,
                           expected_version: i64,
                           command_id: Option<Uuid>)
                           -> Result<PersistResult, WorkflowError> {
    use chrono::Utc;
    use flow::domain::FlowData;
    let key = format!("step_state:{}", step_name);
    // Determinar cursor y versión
    let (cursor_candidate, ev) = self.flow_repo
                                     .get_flow_meta(&self.flow_id)
                                     .map(|meta| {
                                       let version =
                                         if expected_version < 0 { meta.current_version } else { expected_version };
                                       (meta.current_cursor + 1, version)
                                     })
                                     .unwrap_or((0, expected_version)); // Fallback si no hay meta
    let data = FlowData { id: Uuid::new_v4(),
                          flow_id: self.flow_id,
                          cursor: cursor_candidate,
                          key,
                          payload: info.payload,
                          metadata: info.metadata,
                          command_id,
                          created_at: Utc::now() };
    self.flow_repo.persist_data(&data, ev).map_err(Into::into)
  }
}
