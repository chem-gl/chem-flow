// context.rs
//
// Provee `StepContext`, un helper ligero que facilita a los pasos
// acceder a la persistencia (FlowRepository) y a los ports del dominio.
// Incluye utilidades para leer el último payload tipado y para
// persistir resultados tipados de pasos.
use crate::errors::WorkflowError;
use crate::step::constants::key_for_step_state;
use crate::step::StepInfo;
use chem_domain::AllDomainPorts;
use flow::domain::PersistResult;
use flow::repository::FlowRepository;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use uuid::Uuid;
pub struct StepContext {
  pub flow_id: Uuid,
  pub flow_repo: Arc<dyn FlowRepository>,
  pub domain_repo: Arc<dyn AllDomainPorts>,
}
impl StepContext {
  /// Crea un nuevo contexto para el flow indicado.
  pub fn new(flow_id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn AllDomainPorts>) -> Self {
    Self { flow_id, flow_repo, domain_repo }
  }
  // Nota: helper más específico abajo: `get_step_payload_by_name_typed`.
  /// Obtiene el payload del paso `step_name` más reciente, sin tipar.
  pub fn get_step_payload_by_name(&self, step_name: &str) -> Result<Option<serde_json::Value>, WorkflowError> {
    let key = key_for_step_state(step_name);
    let data = self.flow_repo.read_data(&self.flow_id, 0)?;
    for fd in data.iter().rev() {
      if fd.key.eq_ignore_ascii_case(&key) {
        return Ok(Some(fd.payload.clone()));
      }
    }
    Ok(None)
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
    let key = key_for_step_state(step_name);
    // Guardar sin duplicaciones globales: buscar si existe un payload idéntico
    // para este step_name en cualquier cursor del flujo actual. Si existe,
    // retornamos Ok con la versión actual sin insertar.
    {
      let existing = self.flow_repo.read_data(&self.flow_id, 0)?;
      if existing.iter().any(|fd| fd.key.eq_ignore_ascii_case(&key) && fd.payload == info.payload) {
        // no insertar duplicados
        let meta = self.flow_repo.get_flow_meta(&self.flow_id)?;
        return Ok(PersistResult::Ok { new_version: meta.current_version });
      }
    }
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

  /// Obtiene el último payload para un paso por nombre, decodificado a T.
  pub fn get_step_payload_by_name_typed<T>(&self, step_name: &str) -> Result<Option<T>, WorkflowError>
    where T: DeserializeOwned
  {
    let key = key_for_step_state(step_name);
    let data = self.flow_repo.read_data(&self.flow_id, 0)?;
    for fd in data.iter().rev() {
      if fd.key.eq_ignore_ascii_case(&key) {
        if let Ok(v) = serde_json::from_value::<T>(fd.payload.clone()) {
          return Ok(Some(v));
        }
      }
    }
    Ok(None)
  }

  /// Obtiene el último payload de cualquier paso que pueda ser decodificado a
  /// T.
  pub fn get_last_step_payload_any_typed<T>(&self) -> Result<Option<T>, WorkflowError>
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
}
