//! Motor de flujos químicos.
use crate::engine::keys::step_state_key;
use crate::step::{StepContext, StepInfo};
use crate::{workflow_type::WorkflowType, WorkflowError};
use base64::engine::general_purpose::STANDARD as Base64Engine;
use base64::Engine;
use chem_domain::AllDomainPorts;
use chrono::Utc;
use flow::domain::{FlowData, PersistResult};
use flow::repository::FlowRepository;
use serde_json::{json, Value as JsonValue};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

/// Estados posibles del flujo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowStatus {
  NotStarted,
  Running,
  Completed,
  Failed,
  Unknown,
}

/// Trait para motores de flujos químicos.
/// Define la interfaz para gestionar y ejecutar flujos.
pub trait ChemicalFlowEngine: Send + Sync {
  // === Métodos abstractos (deben implementarse) ===
  /// Retorna el ID único del engine.
  fn id(&self) -> Uuid;

  /// Aplica un snapshot al estado interno.
  fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>>;

  /// Genera un snapshot del estado actual.
  fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>>;

  /// Tipo de workflow asociado al engine.
  fn engine_workflow_type() -> WorkflowType
    where Self: Sized;

  /// Construye el engine con repositorios.
  fn construct_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn AllDomainPorts>) -> Self
    where Self: Sized;

  /// Retorna la referencia al repositorio de flows.
  fn flow_repo(&self) -> &Arc<dyn FlowRepository>;

  /// Retorna la referencia al repositorio de dominio.
  fn domain_repo(&self) -> &Arc<dyn AllDomainPorts>;

  /// Obtiene el paso actual como trait object.
  fn get_current_step(&self) -> Result<Box<dyn crate::step::WorkflowStepDyn>, WorkflowError>;

  /// Obtiene el nombre del paso por índice.
  fn step_name_by_index(&self, idx: u32) -> Result<String, WorkflowError>;

  /// Obtiene una instancia del paso por índice.
  fn get_step_by_index(&self, idx: u32) -> Result<Box<dyn crate::step::WorkflowStepDyn>, WorkflowError>;

  // === Métodos con implementación por defecto ===
  /// Ejecuta un paso por índice sin validaciones (modo forzado).
  fn execute_step_by_index_unchecked(&mut self, idx: u32, input: &JsonValue) -> Result<StepInfo, WorkflowError> {
    let step = self.get_step_by_index(idx)?;
    let ctx = StepContext::new(self.id(), self.flow_repo().clone(), self.domain_repo().clone());
    step.execute(&ctx, input)
  }

  /// Crea una nueva instancia del engine.
  fn new(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn AllDomainPorts>) -> Self
    where Self: Sized
  {
    Self::construct_with_repos(id, flow_repo, domain_repo)
  }

  /// Rehidrata el engine desde almacenamiento.
  fn rehydrate(id: Uuid,
               flow_repo: Arc<dyn FlowRepository>,
               domain_repo: Arc<dyn AllDomainPorts>)
               -> Result<Self, WorkflowError>
    where Self: Sized
  {
    let mut engine = Self::new(id, flow_repo, domain_repo);
    engine.rehydrate_from_storage()?;
    Ok(engine)
  }

  /// Crea una nueva rama desde un cursor padre.
  fn new_branch(&self, parent_cursor: i64, metadata: JsonValue) -> Result<Self, WorkflowError>
    where Self: Sized
  {
    let new_id = self.flow_repo()
                     .create_branch(&self.id(), parent_cursor, metadata)
                     .map_err(|e| WorkflowError::Persistence(format!("create_branch error: {}", e)))?;
    let mut new_engine = Self::construct_with_repos(new_id, self.flow_repo().clone(), self.domain_repo().clone());
    new_engine.rehydrate_from_storage()?;
    Ok(new_engine)
  }

  /// Verifica si una rama existe.
  fn branch_exists(&self, flow_id: &Uuid) -> Result<bool, WorkflowError> {
    self.flow_repo().branch_exists(flow_id).map_err(|e| WorkflowError::Persistence(format!("branch_exists error: {}", e)))
  }

  /// Elimina una rama.
  fn delete_branch(&self, flow_id: &Uuid) -> Result<(), WorkflowError> {
    self.flow_repo().delete_branch(flow_id).map_err(|e| WorkflowError::Persistence(format!("delete_branch error: {}", e)))
  }

  /// Obtiene el índice del paso actual.
  fn current_step(&self) -> u32 {
    self.extract_metadata_field("current_step").and_then(|v| v.as_u64()).map(|step| step as u32).unwrap_or(0)
  }

  /// Obtiene el estado actual del flujo.
  fn status(&self) -> FlowStatus {
    self.extract_metadata_field("status")
        .and_then(|v| v.as_str().map(str::to_string))
        .map(|s| match s.as_str() {
          "not_started" => FlowStatus::NotStarted,
          "running" => FlowStatus::Running,
          "completed" => FlowStatus::Completed,
          "failed" => FlowStatus::Failed,
          _ => FlowStatus::Unknown,
        })
        .unwrap_or(FlowStatus::Unknown)
  }

  /// Nombre del paso actual.
  fn current_step_name(&self) -> Result<String, WorkflowError> {
    self.get_current_step().map(|step| step.name().to_string())
  }

  /// Ejecuta el paso actual con entrada JSON.
  fn execute_current_step(&mut self, input: &JsonValue) -> Result<StepInfo, WorkflowError> {
    let step = self.get_current_step()?;
    let step_name = step.name().to_string();
    self.validate_step_execution(&step_name)?;
    let ctx = StepContext::new(self.id(), self.flow_repo().clone(), self.domain_repo().clone());
    step.execute(&ctx, input)
  }

  /// Ejecuta el paso actual con entrada tipada.
  fn execute_current_step_typed<I: serde::Serialize>(&mut self, input: &I) -> Result<StepInfo, WorkflowError> {
    let json_input = serde_json::to_value(input)?;
    self.execute_current_step(&json_input)
  }

  /// Persiste el resultado de un paso.
  fn persist_step_result(&self,
                         step_name: &str,
                         info: StepInfo,
                         expected_version: i64,
                         command_id: Option<Uuid>)
                         -> Result<PersistResult, WorkflowError> {
    let (cursor, version) = self.calculate_cursor_and_version(expected_version)?;
    let data = FlowData { id: Uuid::new_v4(),
                          flow_id: self.id(),
                          cursor,
                          key: step_state_key(step_name),
                          payload: info.payload,
                          metadata: info.metadata,
                          command_id,
                          created_at: Utc::now() };
    let result = self.flow_repo().persist_data(&data, version)?;
    if matches!(result, PersistResult::Ok { .. }) {
      self.update_engine_state_after_persist(data.cursor)?;
    }
    Ok(result)
  }

  /// Avanza al siguiente paso.
  fn advance_step(&mut self) -> Result<(), WorkflowError> {
    self.update_metadata_field("current_step", json!(self.current_step() + 1))
  }

  /// Obtiene el payload del último paso.
  fn get_last_step_payload(&self, step_name: &str) -> Result<Option<JsonValue>, WorkflowError> {
    let key = step_state_key(step_name);
    let data = self.flow_repo().read_data(&self.id(), 0)?;
    let payload = data.into_iter().rev().find(|fd| fd.key.eq_ignore_ascii_case(&key)).map(|fd| fd.payload);
    Ok(payload)
  }

  /// Obtiene metadatos por clave.
  fn get_metadata(&self, key: &str) -> Result<JsonValue, WorkflowError> {
    self.flow_repo().get_meta(&self.id(), key).map_err(|e| WorkflowError::Persistence(format!("get_meta error: {}", e)))
  }

  /// Establece metadatos por clave.
  fn set_metadata(&self, key: &str, value: JsonValue) -> Result<(), WorkflowError> {
    self.flow_repo()
        .set_meta(&self.id(), key, value)
        .map_err(|e| WorkflowError::Persistence(format!("set_meta error: {}", e)))
  }

  /// Valida la ejecución del paso.
  fn validate_step_execution(&self, step_name: &str) -> Result<(), WorkflowError> {
    if self.get_last_step_payload(step_name)?.is_some() {
      return Err(WorkflowError::Validation(format!("El paso '{}' ya fue ejecutado para este flow", step_name)));
    }
    let mut step_idx_opt: Option<u32> = None;
    let mut i: u32 = 0;
    loop {
      match self.step_name_by_index(i) {
        Ok(n) if n == step_name => {
          step_idx_opt = Some(i);
          break;
        }
        Ok(_) => i = i.saturating_add(1),
        Err(_) => break,
      }
    }
    let step_idx = step_idx_opt.ok_or_else(|| {
                                 WorkflowError::Validation(format!("step mapping error: no se encontró el paso '{}'",
                                                                   step_name))
                               })?;
    if step_idx == 0 {
      return Ok(());
    }
    let mut required_steps: Vec<String> =
      (0..step_idx).map(|j| {
                     self.step_name_by_index(j).map_err(|e| WorkflowError::Validation(format!("step mapping error: {}", e)))
                   })
                   .collect::<Result<_, _>>()?;
    required_steps.retain(|s| s != step_name);
    self.ensure_previous_steps_present(&required_steps)
  }

  /// Verifica pasos previos.
  fn ensure_previous_steps_present(&self, required: &[String]) -> Result<(), WorkflowError> {
    let missing: Vec<_> = required.iter()
                                  .filter_map(|req| match self.get_last_step_payload(req) {
                                    Ok(Some(_)) => None,
                                    _ => Some(req.clone()),
                                  })
                                  .collect();
    if missing.is_empty() {
      Ok(())
    } else {
      Err(WorkflowError::Validation(format!("Datos faltantes de pasos previos: {:?}", missing)))
    }
  }

  /// Calcula cursor y versión para persistencia.
  fn calculate_cursor_and_version(&self, expected_version: i64) -> Result<(i64, i64), WorkflowError> {
    match self.flow_repo().get_flow_meta(&self.id()) {
      Ok(meta) => {
        let version = if expected_version < 0 { meta.current_version } else { expected_version };
        Ok((meta.current_cursor + 1, version))
      }
      Err(_) => Ok((0, expected_version)),
    }
  }

  /// Actualiza estado después de persistir.
  fn update_engine_state_after_persist(&self, cursor: i64) -> Result<(), WorkflowError> {
    let next_step = cursor as u32 + 1;
    self.set_metadata("flow_metadata", json!({"current_step": next_step}))?;
    let _ = self.save_snapshot();
    Ok(())
  }

  /// Guarda snapshot.
  fn save_snapshot(&self) -> Result<(), WorkflowError> {
    let snapshot = self.snapshot().map_err(|e| WorkflowError::Persistence(format!("snapshot error: {}", e)))?;
    let state_bytes = serde_json::to_vec(&snapshot)?;
    let state_b64 = Base64Engine.encode(state_bytes);
    self.flow_repo().save_snapshot(&self.id(),
                                    self.current_step() as i64,
                                    &state_b64,
                                    self.get_metadata("flow_metadata")?)?;
    Ok(())
  }

  /// Rehidrata desde almacenamiento.
  fn rehydrate_from_storage(&mut self) -> Result<(), WorkflowError> {
    self.rehydrate_from_snapshot()?;
    self.synchronize_step_state()?;
    Ok(())
  }

  /// Rehidrata desde snapshot.
  fn rehydrate_from_snapshot(&mut self) -> Result<(), WorkflowError> {
    if let Some(snapshot_meta) = self.flow_repo().load_latest_snapshot(&self.id())? {
      if let Ok((bytes, _)) = self.flow_repo().load_snapshot(&snapshot_meta.id) {
        if let Ok(state_b64) = String::from_utf8(bytes) {
          if let Ok(decoded) = Base64Engine.decode(state_b64) {
            let snapshot: JsonValue = serde_json::from_slice(&decoded)?;
            self.apply_snapshot(&snapshot)
                .map_err(|e| WorkflowError::Persistence(format!("apply_snapshot error: {}", e)))?;
          }
        }
      }
    }
    Ok(())
  }

  /// Sincroniza estado del paso.
  fn synchronize_step_state(&mut self) -> Result<(), WorkflowError> {
    match self.get_metadata("flow_metadata") {
      Ok(meta) if !meta.is_null() => self.apply_flow_metadata(meta),
      _ => self.recover_step_from_fallback_sources(),
    }
  }

  /// Aplica metadatos del flujo.
  fn apply_flow_metadata(&mut self, meta: JsonValue) -> Result<(), WorkflowError> {
    if let Some(step) = meta["current_step"].as_u64() {
      self.update_metadata_field("current_step", json!(step))?;
    }
    if let Some(status) = meta["status"].as_str() {
      self.update_metadata_field("status", json!(status))?;
    }
    self.update_metadata_field("flow_metadata", meta)?;
    Ok(())
  }

  /// Recupera paso desde fuentes alternativas.
  fn recover_step_from_fallback_sources(&mut self) -> Result<(), WorkflowError> {
    let step = self.determine_current_step_from_data()?;
    self.update_metadata_field("current_step", json!(step))?;
    Ok(())
  }

  /// Determina paso actual desde datos.
  fn determine_current_step_from_data(&self) -> Result<u32, WorkflowError> {
    if let Ok(data_rows) = self.flow_repo().read_data(&self.id(), -1) {
      if let Some(max_cursor) = data_rows.iter().map(|d| d.cursor).max() {
        return Ok((max_cursor + 1) as u32);
      }
    }
    match self.flow_repo().get_flow_meta(&self.id()) {
      Ok(meta) => Ok(meta.current_cursor as u32 + 1),
      Err(_) => Ok(0),
    }
  }

  /// Extrae campo de metadatos.
  fn extract_metadata_field(&self, field: &str) -> Option<JsonValue> {
    self.get_metadata("flow_metadata").ok().and_then(|meta| meta.get(field).cloned())
  }

  /// Actualiza campo en metadatos.
  fn update_metadata_field(&mut self, field: &str, value: JsonValue) -> Result<(), WorkflowError> {
    let mut obj = self.get_metadata("flow_metadata").unwrap_or_else(|_| json!({})).as_object().cloned().unwrap_or_default();
    obj.insert(field.to_string(), value);
    self.set_metadata("flow_metadata", JsonValue::Object(obj))
  }
}

/// Macro para implementar ChemicalFlowEngine.
#[macro_export]
macro_rules! impl_chemical_flow {
    ($flow_ty:ty, $state_ty:ty, $workflow_type:expr, { $($idx:expr => $step:ty),* $(,)? }) => {
        impl $crate::engine::ChemicalFlowEngine for $flow_ty {
            fn id(&self) -> ::uuid::Uuid {
                self.id
            }

            fn apply_snapshot(&mut self, snapshot: &::serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
                self.state = ::serde_json::from_value(snapshot.clone())?;
                Ok(())
            }

            fn snapshot(&self) -> Result<::serde_json::Value, Box<dyn std::error::Error>> {
                ::serde_json::to_value(&self.state).map_err(Into::into)
            }

            fn engine_workflow_type() -> $crate::workflow_type::WorkflowType {
                $workflow_type
            }

            fn construct_with_repos(
                id: ::uuid::Uuid,
                flow_repo: ::std::sync::Arc<dyn ::flow::repository::FlowRepository>,
                domain_repo: ::std::sync::Arc<dyn ::chem_domain::AllDomainPorts>,
            ) -> Self {
                Self {
                    id,
                    state: Default::default(),
                    flow_repo,
                    domain_repo,
                }
            }

            fn flow_repo(&self) -> &::std::sync::Arc<dyn ::flow::repository::FlowRepository> {
                &self.flow_repo
            }

            fn domain_repo(&self) -> &::std::sync::Arc<dyn ::chem_domain::AllDomainPorts> {
                &self.domain_repo
            }

            fn get_current_step(&self) -> Result<Box<dyn $crate::step::WorkflowStepDyn>, $crate::WorkflowError> {
                self.get_step_by_index(self.state.current_step)
            }

            fn get_step_by_index(&self, idx: u32) -> Result<Box<dyn $crate::step::WorkflowStepDyn>, $crate::WorkflowError> {
                match idx {
                    $($idx => Ok(Box::new(<$step>::default())),)*
                    _ => Err($crate::WorkflowError::Validation("No hay más pasos".into())),
                }
            }

            fn step_name_by_index(&self, idx: u32) -> Result<String, $crate::WorkflowError> {
                match idx {
                    $($idx => Ok(::std::any::type_name::<$step>().rsplitn(2, "::").next().unwrap().to_string()),)*
                    _ => Err($crate::WorkflowError::Validation("No hay más pasos".into())),
                }
            }

            fn apply_flow_metadata(&mut self, meta: ::serde_json::Value) -> Result<(), $crate::WorkflowError> {
                if let Some(step) = meta.get("current_step").and_then(|v| v.as_u64()) {
                    self.state.current_step = step as u32;
                }
                if let Some(status_val) = meta.get("status").and_then(|v| v.as_str()) {
                    self.state.status = status_val.to_string();
                }
                if let Some(obj) = meta.as_object() {
                    self.state.metadata = ::serde_json::Value::Object(obj.clone());
                }
                Ok(())
            }
        }
    };
}
