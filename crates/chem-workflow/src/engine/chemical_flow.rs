// chemical_flow.rs
use crate::step::{StepContext, StepInfo};
use crate::{workflow_type::WorkflowType, WorkflowError};
use base64::Engine;
use chem_domain::DomainRepository;
use chrono::Utc;
use flow::repository::FlowRepository;
use flow::{FlowData, PersistResult};
use serde_json::Value as JsonValue;
use std::{error::Error, sync::Arc};
use uuid::Uuid;
// ========== DEFINICIONES DE TIPOS ==========
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
  // === MÉTODOS REQUERIDOS (IMPLEMENTACIÓN ESPECÍFICA) ===
  /// Identificador único del engine
  fn id(&self) -> Uuid;
  /// Restaura el estado desde un snapshot
  fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>>;
  /// Crea un snapshot del estado actual
  fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>>;
  /// Tipo de workflow específico del engine
  fn engine_workflow_type() -> WorkflowType
    where Self: Sized;
  /// Constructor principal con repositorios inyectados
  fn construct_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized;
  /// Acceso al repositorio de flujo
  fn flow_repo(&self) -> &Arc<dyn FlowRepository>;
  /// Acceso al repositorio de dominio
  fn domain_repo(&self) -> &Arc<dyn DomainRepository>;
  /// Obtiene el paso actual del workflow
  fn get_current_step(&self) -> Result<Box<dyn crate::step::WorkflowStepDyn>, WorkflowError>;
  // === MÉTODOS CON IMPLEMENTACIÓN POR DEFECTO ===
  // --- Constructores ---
  /// Crea una nueva instancia del engine
  fn new(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized
  {
    Self::construct_with_repos(id, flow_repo, domain_repo)
  }
  /// Rehidrata un engine existente desde persistencia
  fn rehydrate(id: Uuid,
               flow_repo: Arc<dyn FlowRepository>,
               domain_repo: Arc<dyn DomainRepository>)
               -> Result<Self, WorkflowError>
    where Self: Sized
  {
    let mut engine = Self::new(id, flow_repo, domain_repo);
    engine.rehydrate_from_storage()?;
    Ok(engine)
  }
  /// Crea una nueva rama a partir del flow actual delegando en el repo
  /// y devuelve una instancia rehidratada del engine apuntando al nuevo
  /// `flow_id`. Esto hace la operación más ergonómica desde el caller.
  fn new_branch(&self, parent_cursor: i64, metadata: JsonValue) -> Result<Self, WorkflowError>
    where Self: Sized
  {
    let new_id = self.flow_repo()
                     .create_branch(&self.id(), parent_cursor, metadata)
                     .map_err(|e| WorkflowError::Persistence(format!("create_branch error: {}", e)))?;
    let mut new = Self::construct_with_repos(new_id, self.flow_repo().clone(), self.domain_repo().clone());
    new.rehydrate_from_storage()?;
    Ok(new)
  }
  /// Conveniencia para delegar a `FlowRepository::branch_exists`
  fn branch_exists(&self, flow_id: &Uuid) -> Result<bool, WorkflowError> {
    self.flow_repo().branch_exists(flow_id).map_err(|e| WorkflowError::Persistence(format!("branch_exists error: {}", e)))
  }
  /// Conveniencia para delegar a `FlowRepository::delete_branch`
  fn delete_branch(&self, flow_id: &Uuid) -> Result<(), WorkflowError> {
    self.flow_repo().delete_branch(flow_id).map_err(|e| WorkflowError::Persistence(format!("delete_branch error: {}", e)))
  }
  // --- Gestión de estado del flow ---
  /// Número del paso actual
  fn current_step(&self) -> u32 {
    self.extract_metadata_field("current_step").and_then(|v| v.as_u64()).map(|step| step as u32).unwrap_or(0)
  }
  /// Estado actual del flujo
  fn status(&self) -> FlowStatus {
    self.extract_metadata_field("status")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .map(|s| match s.as_str() {
          "not_started" => FlowStatus::NotStarted,
          "running" => FlowStatus::Running,
          "completed" => FlowStatus::Completed,
          "failed" => FlowStatus::Failed,
          _ => FlowStatus::Unknown,
        })
        .unwrap_or(FlowStatus::Unknown)
  }
  // --- Ejecución de pasos ---
  /// Nombre del paso actual
  fn current_step_name(&self) -> Result<String, WorkflowError> {
    self.get_current_step().map(|step| step.name().to_string())
  }
  /// Ejecuta el paso actual con entrada JSON
  fn execute_current_step(&mut self, input: &JsonValue) -> Result<StepInfo, WorkflowError> {
    let step = self.get_current_step()?;
    let step_name = step.name().to_string();
    self.validate_step_execution(&step_name)?;
    let ctx = StepContext::new(self.id(), self.flow_repo().clone(), self.domain_repo().clone());
    step.execute(&ctx, input)
  }
  /// Ejecuta el paso actual con entrada tipada
  fn execute_current_step_typed<I: serde::Serialize>(&mut self, input: &I) -> Result<StepInfo, WorkflowError> {
    let json_input = serde_json::to_value(input)?;
    self.execute_current_step(&json_input)
  }
  // --- Persistencia de datos ---
  /// Persiste el resultado de un paso ejecutado
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
    let result = self.flow_repo().persist_data(&data, version)?;
    if let PersistResult::Ok { new_version: _ } = result {
      self.update_engine_state_after_persist(data.cursor)?;
    }
    Ok(result)
  }
  // --- Operaciones de avance y validación ---
  /// Avanza al siguiente paso
  fn advance_step(&mut self) -> Result<(), WorkflowError> {
    self.update_metadata_field("current_step", JsonValue::from(self.current_step() + 1))
  }
  // --- Operaciones de repositorio delegadas ---
  /// Lee el payload del último paso ejecutado
  fn get_last_step_payload(&self, step_name: &str) -> Result<Option<JsonValue>, WorkflowError> {
    let key = format!("step_state:{}", step_name);
    let data = self.flow_repo().read_data(&self.id(), 0)?;
    let payload = data.into_iter().rev().find(|fd| fd.key.eq_ignore_ascii_case(&key)).map(|fd| fd.payload);
    Ok(payload)
  }
  /// Obtiene metadatos específicos
  fn get_metadata(&self, key: &str) -> Result<JsonValue, WorkflowError> {
    self.flow_repo().get_meta(&self.id(), key).map_err(|e| WorkflowError::Persistence(format!("get_meta error: {}", e)))
  }
  /// Establece metadatos
  fn set_metadata(&self, key: &str, value: JsonValue) -> Result<(), WorkflowError> {
    self.flow_repo()
        .set_meta(&self.id(), key, value)
        .map_err(|e| WorkflowError::Persistence(format!("set_meta error: {}", e)))
  }
  // === MÉTODOS PRIVADOS (IMPLEMENTACIÓN INTERNA) ===
  /// Valida que el paso pueda ejecutarse
  fn validate_step_execution(&self, step_name: &str) -> Result<(), WorkflowError> {
    // Verificar que no se re-ejecute un paso ya completado
    if self.get_last_step_payload(step_name)?.is_some() {
      return Err(WorkflowError::Validation(format!("El paso '{}' ya fue ejecutado para este flow", step_name)));
    }
    // Verificar dependencias de pasos previos
    let required_steps: Vec<String> = (0..self.current_step()).map(|i| format!("step{}", i + 1)).collect();
    self.ensure_previous_steps_present(&required_steps)
  }
  /// Verifica que los pasos requeridos estén presentes
  fn ensure_previous_steps_present(&self, required: &[String]) -> Result<(), WorkflowError> {
    let missing: Vec<String> =
      required.iter().filter(|req| self.get_last_step_payload(req).ok().flatten().is_none()).cloned().collect();
    if missing.is_empty() {
      Ok(())
    } else {
      Err(WorkflowError::Validation(format!("Datos faltantes de pasos previos: {:?}", missing)))
    }
  }
  /// Calcula cursor y versión para persistencia
  fn calculate_cursor_and_version(&self, expected_version: i64) -> Result<(i64, i64), WorkflowError> {
    self.flow_repo()
        .get_flow_meta(&self.id())
        .map(|meta| {
          let version = if expected_version < 0 { meta.current_version } else { expected_version };
          (meta.current_cursor + 1, version)
        })
        .or(Ok((0, expected_version)))
  }
  /// Actualiza el estado del engine después de persistir
  fn update_engine_state_after_persist(&self, cursor: i64) -> Result<(), WorkflowError> {
    // Después de persistir un paso con cursor `cursor`, el siguiente
    // paso a ejecutar debe ser `cursor + 1`. Guardamos esa semántica
    // en la metadata `flow_metadata.current_step`.
    let next_step = (cursor as u32).saturating_add(1);
    self.set_metadata("flow_metadata", serde_json::json!({ "current_step": next_step }))?;
    // Intentar guardar snapshot (operación best-effort)
    let _ = self.save_snapshot();
    Ok(())
  }
  /// Guarda snapshot del estado actual
  fn save_snapshot(&self) -> Result<(), WorkflowError> {
    let snapshot = self.snapshot().map_err(|e| WorkflowError::Persistence(format!("snapshot error: {}", e)))?;
    let state_bytes = serde_json::to_vec(&snapshot)?;
    let state_b64 = base64::engine::general_purpose::STANDARD.encode(&state_bytes);
    self.flow_repo().save_snapshot(&self.id(),
                                    self.current_step() as i64,
                                    &state_b64,
                                    self.get_metadata("flow_metadata")?)?;
    Ok(())
  }
  /// Rehidrata el engine desde el almacenamiento
  fn rehydrate_from_storage(&mut self) -> Result<(), WorkflowError> {
    self.rehydrate_from_snapshot()?;
    self.synchronize_step_state()?;
    Ok(())
  }
  /// Rehidrata desde snapshot si está disponible
  fn rehydrate_from_snapshot(&mut self) -> Result<(), WorkflowError> {
    if let Some(snapshot_meta) = self.flow_repo().load_latest_snapshot(&self.id())? {
      let (bytes, _meta) = self.flow_repo().load_snapshot(&snapshot_meta.id)?;
      if let Ok(state_b64) = String::from_utf8(bytes) {
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(state_b64.as_bytes()) {
          let snapshot: JsonValue = serde_json::from_slice(&decoded)?;
          self.apply_snapshot(&snapshot).map_err(|e| WorkflowError::Persistence(format!("apply_snapshot error: {}", e)))?;
        }
      }
    }
    Ok(())
  }
  /// Sincroniza el estado del paso desde metadata o datos persistentes
  fn synchronize_step_state(&mut self) -> Result<(), WorkflowError> {
    match self.get_metadata("flow_metadata") {
      Ok(meta) if !meta.is_null() => self.apply_flow_metadata(meta),
      _ => self.recover_step_from_fallback_sources(),
    }
  }
  /// Aplica metadata del flujo al estado interno
  fn apply_flow_metadata(&mut self, meta: JsonValue) -> Result<(), WorkflowError> {
    // Actualizar el estado interno del engine para reflejar la metadata
    if let Some(step) = meta.get("current_step").and_then(|v| v.as_u64()) {
      // Asegurarnos también de persistir/normalizar el campo en metadata
      self.update_metadata_field("current_step", JsonValue::from(step))?;
    }
    // Si existe status en la metadata, intentar sincronizarlo también
    if let Some(status_val) = meta.get("status").and_then(|v| v.as_str()) {
      self.update_metadata_field("status", JsonValue::from(status_val))?;
    }
    // Guardar copia de metadata completa en el estado del engine si procede
    let _ = self.update_metadata_field("flow_metadata", meta);
    Ok(())
  }
  /// Recupera el estado del paso desde fuentes alternativas
  fn recover_step_from_fallback_sources(&mut self) -> Result<(), WorkflowError> {
    let step = self.determine_current_step_from_data()?;
    // actualizar metadata y estado interno (si aplica)
    self.update_metadata_field("current_step", JsonValue::from(step))?;
    // Actualizar campo interno `state.current_step` cuando esté disponible
    // Algunas implementaciones (generadas por la macro) exponen `state`.
    // Intentamos una actualización conservadora mediante un downcast a Any
    // (no siempre aplicable) — en la mayoría de los casos la macro
    // `impl_chemical_flow!` define `state` y `apply_snapshot` cubrirá
    // las rutas comunes; aquí dejamos sólo la metadata sincronizada.
    Ok(())
  }
  /// Determina el paso actual analizando datos persistentes
  fn determine_current_step_from_data(&self) -> Result<u32, WorkflowError> {
    // Intentar desde flow_data
    if let Ok(data_rows) = self.flow_repo().read_data(&self.id(), -1) {
      if let Some(max_cursor) = data_rows.iter().map(|d| d.cursor).max() {
        // Si el mayor cursor existente es N, el siguiente step es N+1
        return Ok((max_cursor as u32).saturating_add(1));
      }
    }
    // Fallback a metadata del flow
    self.flow_repo().get_flow_meta(&self.id()).map(|meta| (meta.current_cursor as u32).saturating_add(1)).or(Ok(0))
  }
  /// Extrae un campo específico de los metadatos
  fn extract_metadata_field(&self, field: &str) -> Option<JsonValue> {
    self.get_metadata("flow_metadata").ok().and_then(|meta| meta.get(field).cloned())
  }
  /// Actualiza un campo específico en los metadatos
  fn update_metadata_field(&mut self, field: &str, value: JsonValue) -> Result<(), WorkflowError> {
    let mut metadata = self.extract_metadata_field("flow_metadata").and_then(|m| m.as_object().cloned()).unwrap_or_default();
    metadata.insert(field.to_string(), value);
    self.set_metadata("flow_metadata", JsonValue::Object(metadata))
  }
}
// ========== MACRO OPTIMIZADO ==========
/// Macro para implementar ChemicalFlowEngine con mínimo boilerplate
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
                Self { id, state: Default::default(), flow_repo, domain_repo }
            }
            fn flow_repo(&self) -> &::std::sync::Arc<dyn ::flow::repository::FlowRepository> {
                &self.flow_repo
            }
            fn domain_repo(&self) -> &::std::sync::Arc<dyn ::chem_domain::DomainRepository> {
                &self.domain_repo
            }
            fn get_current_step(&self) -> Result<Box<dyn $crate::step::WorkflowStepDyn>, $crate::WorkflowError> {
                match self.state.current_step {
                    $( $idx => Ok(Box::new($step)), )*
                    _ => Err($crate::WorkflowError::Validation("No hay más pasos".into())),
                }
            }
            // Override default to apply metadata into the concrete state
            fn apply_flow_metadata(&mut self, meta: ::serde_json::Value) -> Result<(), $crate::WorkflowError> {
                if let Some(step) = meta.get("current_step").and_then(|v| v.as_u64()) {
                    self.state.current_step = step as u32;
                }
                if let Some(status_val) = meta.get("status").and_then(|v| v.as_str()) {
                    self.state.status = status_val.to_string();
                }
                // If metadata object present, store it into state's metadata field
                if let Some(obj) = meta.as_object() {
                    self.state.metadata = ::serde_json::Value::Object(obj.clone());
                }
                Ok(())
            }
        }
    };
}
