//! Casos de uso del sistema de flujos
//!
//! Los casos de uso encapsulan la lógica de aplicación específica,
//! orquestando las operaciones del dominio y coordinando con los puertos.

use crate::domain::*;
use crate::errors::{FlowError, Result};
use crate::ports::*;
use base64::Engine;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Caso de uso: Crear un nuevo flujo
pub struct CreateFlowUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
  event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl<R> CreateFlowUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>, event_publisher: Option<Arc<dyn EventPublisher>>) -> Self {
    Self { repository, event_publisher }
  }

  pub async fn execute(&self,
                       name: Option<String>,
                       status: Option<String>,
                       metadata: serde_json::Value,
                       initial_step: Option<CreateStepRequest>)
                       -> Result<FlowCreationResult> {
    // Crear metadatos del flujo
    let flow_metadata = FlowMetadata::new(name.clone(),
                                          status.clone(),
                                          None::<String>, // created_by se establecerá externamente
                                          None,           // parent_flow_id
                                          None,           // parent_cursor
                                          metadata);

    // Crear datos iniciales si se proporciona el primer paso
    let initial_data = if let Some(step_req) = initial_step {
      Some(FlowData::new(flow_metadata.id, 1, step_req.key, step_req.payload, step_req.metadata, None))
    } else {
      None
    };

    // Crear flujo en el repositorio
    let flow_id = if let Some(data) = initial_data {
      self.repository.create_flow_with_initial_data(flow_metadata.clone(), data.clone()).await?
    } else {
      self.repository.create_flow(flow_metadata.clone()).await?
    };

    // Publicar evento si hay publisher
    if let Some(publisher) = &self.event_publisher {
      let event = DomainEvent::FlowCreated { flow_id, name: name.clone(), created_by: None, timestamp: Utc::now() };
      let _ = publisher.publish_event(event).await; // No fallar si el evento
                                                    // falla
    }

    Ok(FlowCreationResult { flow_id,
                            name,
                            initial_step_id: None, // Se podría obtener del initial_data si es necesario
                            created_at: flow_metadata.created_at })
  }
}

/// Caso de uso: Añadir un paso a un flujo
pub struct AddStepUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
  event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl<R> AddStepUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>, event_publisher: Option<Arc<dyn EventPublisher>>) -> Self {
    Self { repository, event_publisher }
  }

  pub async fn execute(&self, flow_id: &Uuid, request: AddStepRequest) -> Result<StepCreationResult> {
    // Verificar que el flujo existe
    let flow_metadata = self.repository.get_flow_metadata(flow_id).await?;

    // Calcular siguiente cursor
    let next_cursor = flow_metadata.current_cursor + 1;

    // Crear datos del paso
    let step_data = FlowData::new(*flow_id,
                                  next_cursor,
                                  request.key.clone(),
                                  request.payload,
                                  request.metadata,
                                  request.command_id);

    // Verificar duplicación
    if self.repository.content_exists(&step_data.get_content_hash()).await? {
      return Err(FlowError::Conflict("Duplicate content not allowed".to_string()));
    }

    // Persistir con control optimista
    let result = self.repository.persist_data(&step_data, flow_metadata.current_version).await?;

    match result {
      PersistResult::Ok { new_version: _ } => {
        // Publicar evento
        if let Some(publisher) = &self.event_publisher {
          let event =
            DomainEvent::StepAdded { flow_id: *flow_id, cursor: next_cursor, key: request.key, timestamp: Utc::now() };
          let _ = publisher.publish_event(event).await;
        }

        Ok(StepCreationResult { step_id: step_data.id,
                                cursor: next_cursor,
                                flow_id: *flow_id,
                                created_at: step_data.created_at })
      }
      PersistResult::Conflict => Err(FlowError::Conflict("Version conflict - retry required".to_string())),
    }
  }
}

/// Caso de uso: Crear una rama
pub struct CreateBranchUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
  event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl<R> CreateBranchUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>, event_publisher: Option<Arc<dyn EventPublisher>>) -> Self {
    Self { repository, event_publisher }
  }

  pub async fn execute(&self, request: CreateBranchRequest) -> Result<BranchCreationResult> {
    // Verificar que el flujo padre existe
    let parent_metadata = self.repository.get_flow_metadata(&request.parent_flow_id).await?;

    // Validar cursor
    if request.parent_cursor <= 0 || request.parent_cursor > parent_metadata.current_cursor {
      return Err(FlowError::Conflict(format!("Invalid parent cursor {} for flow with {} steps",
                                             request.parent_cursor, parent_metadata.current_cursor)));
    }

    // Crear rama
    let branch_id = self.repository.create_branch(&request.parent_flow_id, request.parent_cursor, request.metadata).await?;

    // Publicar evento
    if let Some(publisher) = &self.event_publisher {
      let event = DomainEvent::BranchCreated { branch_id,
                                               parent_flow_id: request.parent_flow_id,
                                               parent_cursor: request.parent_cursor,
                                               timestamp: Utc::now() };
      let _ = publisher.publish_event(event).await;
    }

    Ok(BranchCreationResult { branch_id,
                              parent_flow_id: request.parent_flow_id,
                              parent_cursor: request.parent_cursor,
                              created_at: Utc::now() })
  }
}

/// Caso de uso: Eliminar una rama
pub struct DeleteBranchUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
  event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl<R> DeleteBranchUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>, event_publisher: Option<Arc<dyn EventPublisher>>) -> Self {
    Self { repository, event_publisher }
  }

  pub async fn execute(&self, request: DeleteBranchRequest) -> Result<()> {
    // Verificar que la rama existe
    if !self.repository.branch_exists(&request.flow_id).await? {
      return Err(FlowError::NotFound(format!("Branch {} not found", request.flow_id)));
    }

    // Eliminar rama
    self.repository.delete_branch(&request.flow_id, request.recursive).await?;

    // Publicar evento
    if let Some(publisher) = &self.event_publisher {
      let event =
        DomainEvent::BranchDeleted { branch_id: request.flow_id, recursive: request.recursive, timestamp: Utc::now() };
      let _ = publisher.publish_event(event).await;
    }

    Ok(())
  }
}

/// Caso de uso: Obtener path de una rama
pub struct GetBranchPathUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
}

impl<R> GetBranchPathUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>) -> Self {
    Self { repository }
  }

  pub async fn execute(&self, flow_id: &Uuid) -> Result<BranchPathResult> {
    // Verificar que el flujo existe
    let metadata = self.repository.get_flow_metadata(flow_id).await?;

    // Obtener todos los datos del flujo
    let steps = self.repository.read_data(flow_id, 0).await?;

    // Determinar punto de ramificación si es una rama
    let branch_point = if metadata.parent_flow_id.is_some() { metadata.parent_cursor } else { None };

    Ok(BranchPathResult { flow_id: *flow_id, steps, total_steps: metadata.current_cursor as usize, branch_point })
  }
}

/// Caso de uso: Obtener información de un flujo
pub struct GetFlowInfoUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
}

impl<R> GetFlowInfoUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>) -> Self {
    Self { repository }
  }

  pub async fn execute(&self, flow_id: &Uuid) -> Result<FlowInfoResult> {
    // Obtener metadatos
    let metadata = self.repository.get_flow_metadata(flow_id).await?;

    // Contar pasos
    let step_count = self.repository.count_flow_data(flow_id).await?;

    // Obtener ramas hijas
    let child_branches = self.repository.list_child_branches(flow_id).await?;

    // Obtener último snapshot
    let latest_snapshot = self.repository.load_latest_snapshot(flow_id).await?;

    Ok(FlowInfoResult { metadata, step_count, branch_count: child_branches.len(), latest_snapshot, child_branches })
  }
}

/// Caso de uso: Crear snapshot
pub struct CreateSnapshotUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
  blob_storage: Option<Arc<dyn BlobStorage>>,
  event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl<R> CreateSnapshotUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>,
             blob_storage: Option<Arc<dyn BlobStorage>>,
             event_publisher: Option<Arc<dyn EventPublisher>>)
             -> Self {
    Self { repository, blob_storage, event_publisher }
  }

  pub async fn execute(&self, request: CreateSnapshotRequest) -> Result<SnapshotCreationResult> {
    // Obtener estado actual del flujo
    let metadata = self.repository.get_flow_metadata(&request.flow_id).await?;
    let steps = self.repository.read_data(&request.flow_id, 0).await?;

    // Serializar estado
    let state_data = serde_json::to_vec(&steps).map_err(|e| FlowError::Other(format!("Serialization error: {}", e)))?;

    // Guardar en blob storage si está disponible
    let state_key = if let Some(blob_storage) = &self.blob_storage {
      blob_storage.store_blob(&state_data).await?
    } else {
      // Fallback: usar base64 encoding (devuelve String)
      base64::engine::general_purpose::STANDARD.encode(&state_data)
    };

    // Crear snapshot (state_key es String; pasamos &str)
    let snapshot_id =
      self.repository.save_snapshot(&request.flow_id, metadata.current_cursor, &state_key, request.metadata).await?;

    // Publicar evento
    if let Some(publisher) = &self.event_publisher {
      let event = DomainEvent::SnapshotCreated { flow_id: request.flow_id,
                                                 snapshot_id,
                                                 cursor: metadata.current_cursor,
                                                 timestamp: Utc::now() };
      let _ = publisher.publish_event(event).await;
    }

    Ok(SnapshotCreationResult { snapshot_id,
                                cursor: metadata.current_cursor,
                                size_bytes: state_data.len(),
                                created_at: Utc::now() })
  }
}

/// Caso de uso: Rehidratar desde snapshot
pub struct RehydrateFromSnapshotUseCase<R>
  where R: FlowRepository
{
  repository: Arc<R>,
  blob_storage: Option<Arc<dyn BlobStorage>>,
}

impl<R> RehydrateFromSnapshotUseCase<R> where R: FlowRepository
{
  pub fn new(repository: Arc<R>, blob_storage: Option<Arc<dyn BlobStorage>>) -> Self {
    Self { repository, blob_storage }
  }

  pub async fn execute(&self, flow_id: &Uuid) -> Result<RehydrationResult> {
    let start_time = std::time::Instant::now();

    // Obtener último snapshot
    let snapshot_meta = self.repository
                            .load_latest_snapshot(flow_id)
                            .await?
                            .ok_or_else(|| FlowError::NotFound("No snapshots available".to_string()))?;

    // Cargar datos del snapshot
    let snapshot_data = if let Some(blob_storage) = &self.blob_storage {
      blob_storage.retrieve_blob(&snapshot_meta.state_ptr).await?
    } else {
      // Fallback: decodificar base64
      base64::engine::general_purpose::STANDARD.decode(&snapshot_meta.state_ptr)
                                               .map_err(|e| FlowError::Other(format!("Base64 decode error: {}", e)))?
    };

    // Deserializar estado
    let _snapshot_steps: Vec<FlowData> =
      serde_json::from_slice(&snapshot_data).map_err(|e| FlowError::Other(format!("Deserialization error: {}", e)))?;

    // Obtener pasos posteriores al snapshot
    let replay_steps = self.repository.read_data(flow_id, snapshot_meta.cursor).await?;

    // Calcular cursor final
    let final_cursor = if replay_steps.is_empty() { snapshot_meta.cursor } else { replay_steps.last().unwrap().cursor };

    let rehydration_time = start_time.elapsed().as_millis() as u64;

    Ok(RehydrationResult { flow_id: *flow_id,
                           snapshot_cursor: snapshot_meta.cursor,
                           replayed_steps: replay_steps.len(),
                           final_cursor,
                           rehydration_time_ms: rehydration_time })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::adapters::InMemoryFlowRepository;
  use serde_json::json;

  #[tokio::test]
  async fn test_create_flow_use_case() {
    let repo = Arc::new(InMemoryFlowRepository::new());
    let use_case = CreateFlowUseCase::new(repo.clone(), None);

    let result = use_case.execute(Some("Test Flow".to_string()), Some("active".to_string()), json!({}), None).await;

    assert!(result.is_ok());
    let flow_result = result.unwrap();
    assert_eq!(flow_result.name, Some("Test Flow".to_string()));
  }

  #[tokio::test]
  async fn test_add_step_use_case() {
    let repo = Arc::new(InMemoryFlowRepository::new());

    // Crear flujo primero
    let create_use_case = CreateFlowUseCase::new(repo.clone(), None);
    let flow_result =
      create_use_case.execute(Some("Test Flow".to_string()), Some("active".to_string()), json!({}), None).await.unwrap();

    // Añadir paso
    let add_use_case = AddStepUseCase::new(repo.clone(), None);
    let step_request = AddStepRequest { key: "test_step".to_string(),
                                        payload: json!({"content": "test"}),
                                        metadata: json!({}),
                                        command_id: None };

    let result = add_use_case.execute(&flow_result.flow_id, step_request).await;
    assert!(result.is_ok());

    let step_result = result.unwrap();
    assert_eq!(step_result.cursor, 1);
    assert_eq!(step_result.flow_id, flow_result.flow_id);
  }
}
