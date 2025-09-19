use crate::engine::{ChemicalFlowEngine, PersistenceMode, WorkflowConfig};
use crate::errors::WorkflowError;
use crate::step::{StepOutput, WorkflowStep};
use base64::Engine;
use chem_domain::DomainRepository;
use flow::repository::FlowRepository;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

/// Variantes de estado publico expuestas por el flow
#[derive(Debug)]
pub enum FlowStatus {
    /// No iniciado
    NotStarted,
    /// En ejecucion
    Running,
    /// Completado
    Completed,
    /// Fallido
    Failed,
    /// Desconocido
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadmaState {
    /// Paso actual (contador logico, 0-based)
    pub current_step: u32,
    /// Referencias a objetos de dominio producidos por pasos previos
    pub domain_refs: Vec<Uuid>,
    /// Metadata general del flujo (parametros, ultimo resultado, etc.)
    pub metadata: JsonValue,
    /// Estado textual del flujo (ej. "not_started", "running", "completed")
    pub status: String,
}

#[derive(Clone)]
pub struct CadmaFlow {
    pub id: Uuid,
    pub state: CadmaState,
    pub config: WorkflowConfig,
    pub flow_repo: Arc<dyn FlowRepository>,
    pub domain_repo: Arc<dyn DomainRepository>,
}

impl CadmaFlow {
    /// Constructor que toma repositorios de flujo y dominio
    pub fn new(id: Uuid,
               config: WorkflowConfig,
               flow_repo: Arc<dyn FlowRepository>,
               domain_repo: Arc<dyn DomainRepository>)
               -> Self {
        let state = CadmaState { current_step: 0,
                                 domain_refs: Vec::new(),
                                 metadata: JsonValue::Object(serde_json::Map::new()),
                                 status: "not_started".to_string() };
        Self { id,
               state,
               config,
               flow_repo,
               domain_repo }
    }

    /// Alternative constructor used by tests/examples where repos are
    /// provided after creating the flow row.
    pub fn new_with_repos(id: Uuid,
                          config: WorkflowConfig,
                          flow_repo: Arc<dyn FlowRepository>,
                          domain_repo: Arc<dyn DomainRepository>)
                          -> Self {
        Self::new(id, config, flow_repo, domain_repo)
    }

    /// Return the current step index (1-based logical step count)
    pub fn current_step(&self) -> u32 {
        self.state.current_step
    }

    /// Lightweight status enum for callers
    pub fn status(&self) -> FlowStatus {
        match self.state.status.as_str() {
            "not_started" => FlowStatus::NotStarted,
            "running" => FlowStatus::Running,
            "completed" => FlowStatus::Completed,
            "failed" => FlowStatus::Failed,
            _ => FlowStatus::Unknown,
        }
    }

    /// Ejecuta el siguiente paso y persiste el resultado
    pub fn execute_step_and_persist(&mut self) -> Result<(), WorkflowError> {
        let step = self.get_current_step()?;
        let input = self.state.metadata.clone();
        let output = step.execute(&input)?;
        self.persist_output(output)?;
        self.state.current_step += 1;
        if self.state.current_step >= 2 {
            // Assuming 2 steps for CadmaFlow
            self.state.status = "completed".to_string();
        } else {
            self.state.status = "running".to_string();
        }
        Ok(())
    }

    /// Guarda un snapshot del estado actual - placeholder
    pub fn save_snapshot(&self) -> Result<(), WorkflowError> {
        // Serialize state to bytes and call repository save_snapshot.
        let state_bytes = serde_json::to_vec(&self.state)?;
        // For simplicity we store the bytes as a base64 string pointer (state_ptr)
        // but the repository expects a `&str` pointer; here we convert bytes to
        // a string via base64 so the SnapshotStore can decode if needed.
        let state_ptr = base64::engine::general_purpose::STANDARD.encode(&state_bytes);
        let _snapshot_id = self.flow_repo.save_snapshot(&self.id,
                                                         self.state.current_step as i64,
                                                         &state_ptr,
                                                         self.state.metadata.clone())?;
        Ok(())
    }

    /// Rehidrata el estado desde el ultimo snapshot - placeholder
    pub fn rehydrate(&mut self) -> Result<(), WorkflowError> {
        // Try to load latest snapshot metadata
        if let Some(snapshot_meta) = self.flow_repo.load_latest_snapshot(&self.id)? {
            // The stub stores `state_ptr` as a base64 string of the serialized state.
            let state_ptr = snapshot_meta.state_ptr;
            // Decode base64 to bytes
            let bytes =
                base64::engine::general_purpose::STANDARD.decode(state_ptr.as_bytes())
                                                         .map_err(|e| {
                                                             WorkflowError::Persistence(format!("base64 decode error: {}",
                                                                                                e))
                                                         })?;
            // Deserialize CadmaState
            let state: CadmaState = serde_json::from_slice(&bytes)?;
            self.state = state;
            return Ok(());
        }

        // No snapshot: nothing to do for now (could replay events via read_data)
        Ok(())
    }

    /// Crea una rama desde un cursor dado
    pub fn create_branch(&self,
                         parent_cursor: i64,
                         name: Option<String>,
                         status: Option<String>)
                         -> Result<Uuid, WorkflowError> {
        let branch_id = self.flow_repo
                            .create_branch(&self.id, name, status, parent_cursor, JsonValue::Null)?;
        Ok(branch_id)
    }

    /// Limpia pasos desde un cursor (prune) - placeholder, asume metodo no
    /// existe
    pub fn clean_from_step(&self, _cursor: i64) -> Result<(), WorkflowError> {
        // Placeholder: implement if method exists in FlowRepository
        Err(WorkflowError::Persistence("delete_steps_from_cursor not implemented".to_string()))
    }

    fn get_current_step(&self) -> Result<Box<dyn WorkflowStep>, WorkflowError> {
        match self.state.current_step {
            0 => Ok(Box::new(super::steps::Step1)),
            1 => Ok(Box::new(super::steps::Step2)),
            _ => Err(WorkflowError::Validation("No more steps".to_string())),
        }
    }

    fn persist_output(&mut self, output: StepOutput) -> Result<(), WorkflowError> {
        // Update state with output
        self.state.domain_refs.extend(output.produced_domain_refs.clone());
        self.state.metadata = output.metadata;

        // Persist domain objects if SeparateTables
        if matches!(self.config.persistence_mode, PersistenceMode::SeparateTables) {
            // Placeholder: save molecules/families to domain_repo
            // For now, assume refs are handled externally
        }

        // Always persist FlowData
        let flow_data = flow::domain::FlowData { id: Uuid::new_v4(),
                                                 flow_id: self.id,
                                                 cursor: self.state.current_step as i64,
                                                 key: format!("step{}", self.state.current_step),
                                                 payload: output.payload,
                                                 metadata: self.state.metadata.clone(),
                                                 created_at: chrono::Utc::now(),
                                                 command_id: None };
        self.flow_repo
            .persist_data(&flow_data, (self.state.current_step as i64).saturating_sub(1))?;
        Ok(())
    }
}

impl ChemicalFlowEngine for CadmaFlow {
    fn id(&self) -> Uuid {
        self.id
    }

    fn execute_next(&mut self) -> Result<JsonValue, Box<dyn Error>> {
        self.execute_step_and_persist()?;
        Ok(self.state.metadata.clone())
    }

    fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>> {
        self.state = serde_json::from_value(snapshot.clone())?;
        Ok(())
    }

    fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>> {
        Ok(serde_json::to_value(&self.state)?)
    }

    fn create_branch(&self,
                     parent_cursor: i64,
                     name: Option<String>,
                     status: Option<String>)
                     -> Result<Uuid, Box<dyn Error>> {
        Ok(self.create_branch(parent_cursor, name, status)?)
    }

    fn config(&self) -> WorkflowConfig {
        self.config.clone()
    }
}
