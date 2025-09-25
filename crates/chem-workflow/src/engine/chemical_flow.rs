// chemical_flow.rs
use crate::step::{StepContext, StepInfo};
use crate::{workflow_type::WorkflowType, WorkflowError};
use base64::Engine;
use chem_domain::DomainRepository;
use chrono::Utc;
use flow::repository::FlowRepository;
use flow::{FlowData, PersistResult};
use serde_json::{Map, Value as JsonValue};
use std::{error::Error, sync::Arc};
use uuid::Uuid;

/// Estado público común a todos los engines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowStatus {
  NotStarted,
  Running,
  Completed,
  Failed,
  Unknown,
}

// ========== TRAIT PRINCIPAL ==========

/// Trait genérico para motores de flujo químicos
pub trait ChemicalFlowEngine: Send + Sync {
  // === Métodos requeridos (deben ser implementados) ===
  fn id(&self) -> Uuid;
  fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>>;
  fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>>;
  fn engine_workflow_type() -> WorkflowType
    where Self: Sized;
  fn construct_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized;
  fn flow_repo(&self) -> &Arc<dyn FlowRepository>;
  fn domain_repo(&self) -> &Arc<dyn DomainRepository>;
  fn get_current_step(&self) -> Result<Box<dyn crate::step::WorkflowStepDyn>, WorkflowError>;

  // === Métodos con implementación por defecto ===

  // Constructores
  fn new(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized
  {
    Self::construct_with_repos(id, flow_repo, domain_repo)
  }

  fn new_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized
  {
    Self::new(id, flow_repo, domain_repo)
  }

  fn rehydrate_with_repos(id: Uuid,
                          flow_repo: Arc<dyn FlowRepository>,
                          domain_repo: Arc<dyn DomainRepository>)
                          -> Result<Self, WorkflowError>
    where Self: Sized
  {
    let mut engine = Self::new_with_repos(id, flow_repo, domain_repo);
    engine.rehydrate()?;
    Ok(engine)
  }

  // Gestión de estado del flow
  fn current_step(&self) -> u32 {
    self.get_flow_metadata()
        .ok()
        .and_then(|meta| meta.get("current_step").and_then(|v| v.as_u64()))
        .map(|step| step as u32)
        .unwrap_or(0)
  }

  fn get_flow_metadata(&self) -> Result<JsonValue, WorkflowError> {
    self.get_metadata("flow_metadata")
  }

  fn set_flow_metadata(&mut self, metadata: JsonValue) -> Result<(), WorkflowError> {
    self.set_metadata("flow_metadata", metadata)
  }

  fn set_current_step(&mut self, step: u32) -> Result<(), WorkflowError> {
    self.update_metadata_field("current_step", JsonValue::from(step))
  }

  fn set_flow_status(&mut self, status: String) -> Result<(), WorkflowError> {
    self.update_metadata_field("status", JsonValue::from(status))
  }

  fn status(&self) -> FlowStatus {
    if let Ok(meta) = self.get_flow_metadata() {
      if let Some(s) = meta.get("status").and_then(|v| v.as_str()) {
        return match s {
          "not_started" => FlowStatus::NotStarted,
          "running" => FlowStatus::Running,
          "completed" => FlowStatus::Completed,
          "failed" => FlowStatus::Failed,
          _ => FlowStatus::Unknown,
        };
      }
    }
    FlowStatus::Unknown
  }

  // Ejecución de pasos
  fn current_step_name(&self) -> Result<String, WorkflowError> {
    self.get_current_step().map(|step| step.name().to_string())
  }

  fn execute_current_step(&mut self, input: &JsonValue) -> Result<StepInfo, WorkflowError> {
    let step = self.get_current_step()?;
    let required_steps: Vec<String> = (0..self.current_step()).map(|i| format!("step{}", i + 1)).collect();

    self.ensure_previous_steps_present(&required_steps)?;

    let ctx = StepContext::new(self.id(), self.flow_repo().clone(), self.domain_repo().clone());
    // Evitar re-ejecución del mismo paso si ya existe un payload para él
    let step_name = step.name().to_string();
    if self.get_last_step_payload(&step_name)?.is_some() {
      return Err(WorkflowError::Validation(format!("El paso '{}' ya fue ejecutado para este flow", step_name)));
    }

    step.execute(&ctx, input)
  }

  fn execute_current_step_typed<I: serde::Serialize>(&mut self, input: &I) -> Result<StepInfo, WorkflowError> {
    let json_input = serde_json::to_value(input)?;
    self.execute_current_step(&json_input)
  }

  // Persistencia de datos
  fn get_last_step_payload(&self, step_name: &str) -> Result<Option<JsonValue>, WorkflowError> {
    let key = format!("step_state:{}", step_name);
    let data = self.flow_repo().read_data(&self.id(), 0)?;
    let opt = data.into_iter().rev().find(|fd| fd.key.eq_ignore_ascii_case(&key)).map(|fd| fd.payload);
    Ok(opt)
  }

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
                          key: format!("step_state:{}", step_name),
                          payload: info.payload,
                          metadata: info.metadata,
                          command_id,
                          created_at: Utc::now() };

    // Persist the flow data. If successful, also persist a lightweight
    // `flow_metadata` entry with `current_step` pointing to the just
    // written cursor so engines can rehydrate the current step.
    let pr = match self.flow_repo().persist_data(&data, version) {
      Ok(v) => v,
      Err(e) => return Err(e.into()),
    };
    // Only update metadata when the persist was accepted (Ok).
    match pr {
      PersistResult::Ok { new_version: _ } => {
        // Best-effort: ignore metadata errors but prefer to surface
        // them as warnings in logs (we cannot log here, so ignore).
        // Persist a lightweight metadata entry indicating the current
        // step we just wrote. This helps rehydrate to quickly recover
        // the engine cursor without scanning flow_data.
        let _ = self.set_metadata("flow_metadata", serde_json::json!({ "current_step": data.cursor }));
        // Additionally, attempt to persist a full snapshot of the
        // engine state so rehydration can use it to restore the
        // complete in-memory state (best-effort; do not fail the
        // step persist if snapshot saving fails).
        let _ = self.save_snapshot();
      }
      _ => {}
    }
    Ok(pr)
  }

  // Gestión de avance y validación
  fn advance_step(&mut self) -> Result<(), WorkflowError> {
    self.set_current_step(self.current_step().saturating_add(1))
  }

  fn ensure_previous_steps_present(&self, required: &[String]) -> Result<(), WorkflowError> {
    let missing: Vec<String> =
      required.iter().filter(|req| self.get_last_step_payload(req).ok().flatten().is_none()).cloned().collect();

    if missing.is_empty() {
      Ok(())
    } else {
      Err(WorkflowError::Validation(format!("Datos faltantes de pasos previos: {:?}", missing)))
    }
  }

  // Cálculos internos
  fn calculate_cursor_and_version(&self, expected_version: i64) -> Result<(i64, i64), WorkflowError> {
    if let Ok(meta) = self.flow_repo().get_flow_meta(&self.id()) {
      let version = if expected_version < 0 { meta.current_version } else { expected_version };
      Ok((meta.current_cursor + 1, version))
    } else {
      Ok((0, expected_version))
    }
  }

  // Snapshots
  fn save_snapshot(&self) -> Result<(), WorkflowError> {
    let snapshot = self.snapshot().map_err(|e| WorkflowError::Persistence(format!("snapshot error: {}", e)))?;

    let state_bytes = serde_json::to_vec(&snapshot)?;
    let state_ptr = base64::engine::general_purpose::STANDARD.encode(&state_bytes);

    self.flow_repo().save_snapshot(&self.id(), self.current_step() as i64, &state_ptr, self.get_flow_metadata()?)?;

    Ok(())
  }

  fn rehydrate(&mut self) -> Result<(), WorkflowError> {
    if let Some(snapshot_meta) = self.flow_repo().load_latest_snapshot(&self.id())? {
      let (bytes, _meta) = self.flow_repo().load_snapshot(&snapshot_meta.id)?;

      if let Ok(state_b64) = String::from_utf8(bytes) {
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(state_b64.as_bytes()) {
          let snapshot: JsonValue = serde_json::from_slice(&decoded)?;
          self.apply_snapshot(&snapshot).map_err(|e| WorkflowError::Persistence(format!("apply_snapshot error: {}", e)))?;
        }
      }
    }
    // Además de intentar rehidratar desde snapshot, cargar la metadata
    // específica `flow_metadata` desde el repositorio para asegurar que
    // campos como `current_step` se reflejen en el estado interno del
    // engine incluso si no hay snapshot disponible.
    match self.get_flow_repo().get_meta(&self.id(), "flow_metadata") {
      Ok(meta_val) => {
        if !meta_val.is_null() {
          // Ignorar errores al aplicar metadata; no queremos fallar la
          // rehidratación completa sólo por un problema de metadata
          let _ = self.set_flow_metadata(meta_val.clone());
          // Además de almacenar la metadata, si contiene el campo
          // `current_step` debemos sincronizar el estado interno
          // del engine (p. ej. `state.current_step` en engines
          // generados por la macro). Esto evita que la metadata
          // persista pero el engine siga con paso 0 en memoria.
          if let Some(cs) = meta_val.get("current_step").and_then(|v| v.as_u64()) {
            let _ = self.set_current_step(cs as u32);
          }
        } else {
          // Si no existe la key `flow_metadata`, intentar inferir el
          // paso actual a partir de los `FlowData` ya persistidos. Esto
          // cubre casos donde hubo registros en `flow_data` pero por
          // alguna razón `flows.current_cursor` no se actualizó.
          if let Ok(data_rows) = self.get_flow_repo().read_data(&self.id(), -1) {
            if !data_rows.is_empty() {
              if let Some(max_cursor) = data_rows.iter().map(|d| d.cursor).max() {
                let cs = max_cursor as u32;
                // Actualizar el estado interno del engine para que
                // `current_step()` refleje los pasos ya persistidos.
                let _ = self.set_current_step(cs);
              }
            } else if let Ok(flow_meta) = self.get_flow_repo().get_flow_meta(&self.id()) {
              // Fallback final: usar current_cursor si no hay rows
              let cs = flow_meta.current_cursor as u32;
              let _ = self.set_current_step(cs);
            }
          } else if let Ok(flow_meta) = self.get_flow_repo().get_flow_meta(&self.id()) {
            let cs = flow_meta.current_cursor as u32;
            let meta_json = serde_json::json!({ "current_step": cs });
            let _ = self.set_flow_metadata(meta_json);
          }
        }
      }
      Err(_) => {
        // Si no podemos leer la key, intentar fallback igualmente
          if let Ok(flow_meta) = self.get_flow_repo().get_flow_meta(&self.id()) {
            let cs = flow_meta.current_cursor as u32;
            let _ = self.set_current_step(cs);
          }
      }
    }
    Ok(())
  }

  // Operaciones de repositorio delegadas
  fn get_flow_repo(&self) -> Arc<dyn FlowRepository> {
    self.flow_repo().clone()
  }

  fn get_metadata(&self, key: &str) -> Result<JsonValue, WorkflowError> {
    self.flow_repo().get_meta(&self.id(), key).map_err(|e| WorkflowError::Persistence(format!("get_meta error: {}", e)))
  }

  fn set_metadata(&self, key: &str, value: JsonValue) -> Result<(), WorkflowError> {
    self.flow_repo()
        .set_meta(&self.id(), key, value)
        .map_err(|e| WorkflowError::Persistence(format!("set_meta error: {}", e)))
  }

  fn del_metadata(&self, key: &str) -> Result<(), WorkflowError> {
    self.flow_repo().del_meta(&self.id(), key).map_err(|e| WorkflowError::Persistence(format!("del_meta error: {}", e)))
  }

  fn count_steps_initialized(&self) -> Result<i64, WorkflowError> {
    self.flow_repo().count_steps(&self.id()).map_err(|e| WorkflowError::Persistence(format!("count_steps error: {}", e)))
  }

  // Helper interno para actualizar metadata
  fn update_metadata_field(&mut self, field: &str, value: JsonValue) -> Result<(), WorkflowError> {
    let mut metadata_map = match self.get_flow_metadata() {
      Ok(JsonValue::Object(map)) => map,
      _ => Map::new(),
    };

    metadata_map.insert(field.to_string(), value);
    self.set_flow_metadata(JsonValue::Object(metadata_map))
  }
}

// ========== MACRO PARA IMPLEMENTACIÓN ==========

/// Macro para implementar ChemicalFlowEngine de manera boilerplate-free
#[macro_export]
macro_rules! impl_chemical_flow {
    ($flow_ty:ty, $state_ty:ty, $workflow_type:expr, { $($idx:expr => $step:path),* $(,)? }) => {
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
                domain_repo: ::std::sync::Arc<dyn ::chem_domain::DomainRepository>
            ) -> Self {
                Self {
                    id,
                    state: ::std::default::Default::default(),
                    flow_repo,
                    domain_repo,
                }
            }

            fn flow_repo(&self) -> &::std::sync::Arc<dyn ::flow::repository::FlowRepository> {
                &self.flow_repo
            }

            fn domain_repo(&self) -> &::std::sync::Arc<dyn ::chem_domain::DomainRepository> {
                &self.domain_repo
            }

            fn get_current_step(&self) -> Result<Box<dyn $crate::step::WorkflowStepDyn>, $crate::errors::WorkflowError> {
                match self.state.current_step {
                    $( $idx => Ok(Box::new($step)), )*
                    _ => Err($crate::errors::WorkflowError::Validation(
                        "No hay más pasos".to_string()
                    )),
                }
            }

            // Implementaciones específicas para estado interno
            fn current_step(&self) -> u32 {
                self.state.current_step
            }

            fn get_flow_metadata(&self) -> Result<::serde_json::Value, $crate::errors::WorkflowError> {
                Ok(self.state.metadata.clone())
            }

            fn set_flow_metadata(&mut self, metadata: ::serde_json::Value) -> Result<(), $crate::errors::WorkflowError> {
                self.state.metadata = metadata;
                Ok(())
            }

      fn set_current_step(&mut self, step: u32) -> Result<(), $crate::errors::WorkflowError> {
        use ::serde_json::Value as JsonValue;
        self.state.current_step = step;
        // Update in-memory metadata
        let mut meta = self.state.metadata.clone();
        if !meta.is_object() {
          meta = ::serde_json::json!({});
        }
        if let Some(obj) = meta.as_object_mut() {
          obj.insert("current_step".to_string(), JsonValue::from(step));
        }
        self.state.metadata = meta.clone();
        // Persist metadata to repository; propagate any persistence errors
        match self.flow_repo.set_meta(&self.id, "flow_metadata", meta) {
          Ok(()) => Ok(()),
          Err(e) => Err($crate::errors::WorkflowError::Persistence(format!("set_meta error: {}", e))),
        }
      }
            fn set_flow_status(&mut self, status: String) -> Result<(), $crate::errors::WorkflowError> {
                self.state.status = status;
                Ok(())
            }
        }
    };
}
