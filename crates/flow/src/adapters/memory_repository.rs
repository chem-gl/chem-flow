//! Adaptador de repositorio en memoria
//!
//! Implementación en memoria de todos los puertos de repositorio para pruebas
//! y desarrollo. Mantiene compatibilidad con la API existente mientras
//! implementa la nueva arquitectura.

use crate::domain::value_objects::*;
use crate::errors::{FlowError, Result};
use crate::ports::outbound::*;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Repositorio en memoria que implementa todos los puertos necesarios
pub struct InMemoryFlowRepository {
  /// Metadatos de flows indexados por `flow_id`
  flows: Arc<Mutex<HashMap<Uuid, FlowMetadata>>>,
  /// Registros de `FlowData` por flow (ordenados por inserción/`cursor`)
  steps: Arc<Mutex<HashMap<Uuid, Vec<FlowData>>>>,
  /// Snapshots metadata indexados por snapshot id
  snapshots: Arc<Mutex<HashMap<Uuid, SnapshotMeta>>>,
  /// Datos de blobs simulados
  blobs: Arc<Mutex<HashMap<String, Vec<u8>>>>,
  /// Metadatos específicos por flujo
  metadata_store: Arc<Mutex<HashMap<(Uuid, String), Value>>>,
}

impl InMemoryFlowRepository {
  pub fn new() -> Self {
    Self { flows: Arc::new(Mutex::new(HashMap::new())),
           steps: Arc::new(Mutex::new(HashMap::new())),
           snapshots: Arc::new(Mutex::new(HashMap::new())),
           blobs: Arc::new(Mutex::new(HashMap::new())),
           metadata_store: Arc::new(Mutex::new(HashMap::new())) }
  }

  /// Helper para mapear errores de Mutex
  fn handle_poison<T>(&self, result: std::sync::LockResult<T>) -> Result<T> {
    result.map_err(|e| FlowError::Storage(format!("Mutex poisoned: {:?}", e)))
  }

  /// Reemplaza las entradas del metadata_store por las del Value provisto
  fn reset_metadata_entries(&self, flow_id: &Uuid, metadata: &Value) -> Result<()> {
    let mut metadata_store = self.handle_poison(self.metadata_store.lock())?;
    metadata_store.retain(|(id, _), _| id != flow_id);
    if let Value::Object(map) = metadata {
      for (key, value) in map {
        metadata_store.insert((*flow_id, key.clone()), value.clone());
      }
    }
    Ok(())
  }

  /// Elimina todas las entradas de metadata_store asociadas al flow_id
  fn clear_metadata_entries(&self, flow_id: &Uuid) -> Result<()> {
    let mut metadata_store = self.handle_poison(self.metadata_store.lock())?;
    metadata_store.retain(|(id, _), _| id != flow_id);
    Ok(())
  }
}

impl Default for InMemoryFlowRepository {
  fn default() -> Self {
    Self::new()
  }
}

// Implementación del puerto principal
#[async_trait]
impl FlowRepository for InMemoryFlowRepository {
  async fn create_flow_with_initial_data(&self, metadata: FlowMetadata, initial_data: FlowData) -> Result<Uuid> {
    let flow_id = metadata.id;
    let metadata_value = metadata.metadata.clone();

    // Crear flujo
    {
      let mut flows = self.handle_poison(self.flows.lock())?;
      flows.insert(flow_id, metadata);
    }

    // Añadir datos iniciales si se proporcionan
    {
      let mut steps = self.handle_poison(self.steps.lock())?;
      steps.insert(flow_id, vec![initial_data]);
    }

    // Sincronizar metadata expuesta por clave
    self.reset_metadata_entries(&flow_id, &metadata_value)?;

    Ok(flow_id)
  }

  async fn get_repository_stats(&self) -> Result<RepositoryStats> {
    let flows = self.handle_poison(self.flows.lock())?;
    let steps = self.handle_poison(self.steps.lock())?;
    let snapshots = self.handle_poison(self.snapshots.lock())?;
    let blobs = self.handle_poison(self.blobs.lock())?;

    let total_data_records = steps.values().map(|v| v.len()).sum();
    let storage_size_bytes = blobs.values().map(|v| v.len() as u64).sum();

    // Contar ramas (flujos con parent_flow_id)
    let total_branches = flows.values().filter(|f| f.parent_flow_id.is_some()).count();

    Ok(RepositoryStats { total_flows: flows.len(),
                         total_data_records,
                         total_snapshots: snapshots.len(),
                         total_branches,
                         storage_size_bytes })
  }
}

// Implementación del puerto de metadatos de flujos
#[async_trait]
impl FlowMetadataPort for InMemoryFlowRepository {
  async fn get_flow_metadata(&self, flow_id: &Uuid) -> Result<FlowMetadata> {
    let flows = self.handle_poison(self.flows.lock())?;
    flows.get(flow_id).cloned().ok_or_else(|| FlowError::NotFound(format!("Flow {}", flow_id)))
  }

  async fn create_flow(&self, metadata: FlowMetadata) -> Result<Uuid> {
    let flow_id = metadata.id;
    let metadata_value = metadata.metadata.clone();
    {
      let mut flows = self.handle_poison(self.flows.lock())?;
      flows.insert(flow_id, metadata);
    }
    self.reset_metadata_entries(&flow_id, &metadata_value)?;
    Ok(flow_id)
  }

  async fn update_flow_metadata(&self, flow_id: &Uuid, metadata: FlowMetadata) -> Result<()> {
    let metadata_value = metadata.metadata.clone();
    {
      let mut flows = self.handle_poison(self.flows.lock())?;
      if flows.contains_key(flow_id) {
        flows.insert(*flow_id, metadata);
      } else {
        return Err(FlowError::NotFound(format!("Flow {}", flow_id)));
      }
    }
    self.reset_metadata_entries(flow_id, &metadata_value)?;
    Ok(())
  }

  async fn delete_flow(&self, flow_id: &Uuid) -> Result<()> {
    let mut flows = self.handle_poison(self.flows.lock())?;
    let mut steps = self.handle_poison(self.steps.lock())?;
    let mut snapshots = self.handle_poison(self.snapshots.lock())?;

    flows.remove(flow_id);
    steps.remove(flow_id);

    // Eliminar snapshots relacionados
    let snapshot_ids: Vec<Uuid> = snapshots.iter().filter(|(_, s)| s.flow_id == *flow_id).map(|(k, _)| *k).collect();

    for id in snapshot_ids {
      snapshots.remove(&id);
    }

    // Limpiar metadatos asociados
    self.clear_metadata_entries(flow_id)?;

    Ok(())
  }

  async fn list_flow_ids(&self) -> Result<Vec<Uuid>> {
    let flows = self.handle_poison(self.flows.lock())?;
    Ok(flows.keys().cloned().collect())
  }

  async fn flow_exists(&self, flow_id: &Uuid) -> Result<bool> {
    let flows = self.handle_poison(self.flows.lock())?;
    Ok(flows.contains_key(flow_id))
  }

  async fn get_flow_status(&self, flow_id: &Uuid) -> Result<Option<String>> {
    let flows = self.handle_poison(self.flows.lock())?;
    Ok(flows.get(flow_id).and_then(|m| m.status.clone()))
  }

  async fn set_flow_status(&self, flow_id: &Uuid, status: Option<String>) -> Result<()> {
    let mut flows = self.handle_poison(self.flows.lock())?;
    if let Some(metadata) = flows.get_mut(flow_id) {
      metadata.status = status;
      Ok(())
    } else {
      Err(FlowError::NotFound(format!("Flow {}", flow_id)))
    }
  }
}

// Implementación del puerto de datos de flujos
#[async_trait]
impl FlowDataPort for InMemoryFlowRepository {
  async fn persist_data(&self, data: &FlowData, expected_version: i64) -> Result<PersistResult> {
    let mut flows = self.handle_poison(self.flows.lock())?;
    let mut steps = self.handle_poison(self.steps.lock())?;

    let flow_metadata = flows.get_mut(&data.flow_id).ok_or_else(|| FlowError::NotFound("Flow not found".to_string()))?;

    // Control optimista
    if flow_metadata.current_version != expected_version {
      return Ok(PersistResult::Conflict);
    }

    // Verificar idempotencia
    if let Some(cmd_id) = data.command_id {
      if let Some(existing) = steps.get(&data.flow_id) {
        if existing.iter().any(|d| d.command_id == Some(cmd_id)) {
          return Ok(PersistResult::Ok { new_version: flow_metadata.current_version });
        }
      }
    }

    // Validar cursor
    if data.cursor <= flow_metadata.current_cursor {
      return Err(FlowError::Conflict(format!("Cursor {} not greater than current {}",
                                             data.cursor, flow_metadata.current_cursor)));
    }

    // Persistir datos
    let list = steps.entry(data.flow_id).or_default();
    list.push(data.clone());

    // Actualizar metadata
    flow_metadata.current_version += 1;
    flow_metadata.current_cursor = data.cursor;

    Ok(PersistResult::Ok { new_version: flow_metadata.current_version })
  }

  async fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> Result<Vec<FlowData>> {
    let steps = self.handle_poison(self.steps.lock())?;
    Ok(steps.get(flow_id).cloned().unwrap_or_default().into_iter().filter(|d| d.cursor > from_cursor).collect())
  }

  async fn read_data_at_cursor(&self, flow_id: &Uuid, cursor: i64) -> Result<Option<FlowData>> {
    let steps = self.handle_poison(self.steps.lock())?;
    Ok(steps.get(flow_id).and_then(|list| list.iter().find(|d| d.cursor == cursor)).cloned())
  }

  async fn count_flow_data(&self, flow_id: &Uuid) -> Result<i64> {
    let flows = self.handle_poison(self.flows.lock())?;
    if let Some(metadata) = flows.get(flow_id) {
      Ok(metadata.current_cursor)
    } else {
      Ok(-1) // Flujo no existe
    }
  }

  async fn delete_data_from_cursor(&self, flow_id: &Uuid, from_cursor: i64) -> Result<()> {
    // Remove steps from from_cursor and update metadata current_cursor
    {
      let mut steps = self.handle_poison(self.steps.lock())?;
      if let Some(list) = steps.get_mut(flow_id) {
        list.retain(|d| d.cursor < from_cursor);
      }
    }

    // Update current_cursor to max remaining or 0
    let max_cursor = {
      let steps = self.handle_poison(self.steps.lock())?;
      steps.get(flow_id).map(|list| list.iter().map(|d| d.cursor).max().unwrap_or(0)).unwrap_or(0)
    };
    {
      let mut flows = self.handle_poison(self.flows.lock())?;
      if let Some(meta) = flows.get_mut(flow_id) {
        meta.current_cursor = max_cursor;
      }
    }

    // Collect child_ids to delete (whose parent_cursor >= from_cursor)
    let child_ids_to_delete = {
      let child_ids = self.list_child_branches(flow_id).await?;
      let flows = self.handle_poison(self.flows.lock())?;
      child_ids.into_iter()
               .filter(|child_id| {
                 flows.get(child_id).map(|child_meta| child_meta.parent_cursor.unwrap_or(0) >= from_cursor).unwrap_or(false)
               })
               .collect::<Vec<_>>()
    };

    // Now delete branches without holding the lock
    for child_id in child_ids_to_delete {
      self.delete_branch(&child_id, true).await?;
    }

    Ok(())
  }

  async fn content_exists(&self, content_hash: &str) -> Result<bool> {
    let steps = self.handle_poison(self.steps.lock())?;
    for list in steps.values() {
      for data in list {
        if data.get_content_hash() == content_hash {
          return Ok(true);
        }
      }
    }
    Ok(false)
  }
}

// Implementación del puerto de gestión de ramas
#[async_trait]
impl BranchManagementPort for InMemoryFlowRepository {
  async fn create_branch(&self, parent_flow_id: &Uuid, parent_cursor: i64, metadata: Value) -> Result<Uuid> {
    let new_id = Uuid::new_v4();

    // Obtener metadata del padre
    let parent_metadata = {
      let flows = self.handle_poison(self.flows.lock())?;
      flows.get(parent_flow_id).cloned()
    };

    // Crear metadata de la nueva rama
    let branch_metadata = if let Some(mut pm) = parent_metadata {
      pm.id = new_id;
      pm.parent_flow_id = Some(*parent_flow_id);
      pm.parent_cursor = Some(parent_cursor);
      pm.current_cursor = parent_cursor;
      pm.current_version = 0;
      pm.metadata = metadata;
      pm
    } else {
      FlowMetadata::new(Some(format!("branch-of-{}", parent_flow_id)),
                        Some("queued".to_string()),
                        None::<String>,
                        Some(*parent_flow_id),
                        Some(parent_cursor),
                        metadata)
    };

    let metadata_value = branch_metadata.metadata.clone();

    // Insertar metadata
    {
      let mut flows = self.handle_poison(self.flows.lock())?;
      flows.insert(new_id, branch_metadata);
    }

    self.reset_metadata_entries(&new_id, &metadata_value)?;

    // Copiar pasos del padre hasta parent_cursor
    {
      let mut steps = self.handle_poison(self.steps.lock())?;
      if let Some(parent_steps) = steps.get(parent_flow_id).cloned() {
        let copied: Vec<FlowData> =
          parent_steps.into_iter().filter(|d| d.cursor <= parent_cursor).map(|d| d.clone_for_branch(new_id)).collect();

        steps.insert(new_id, copied);
      }

      // Defensive: ensure no steps beyond parent_cursor remain
      if let Some(vec) = steps.get_mut(&new_id) {
        vec.retain(|d| d.cursor <= parent_cursor);
      }
    }

    Ok(new_id)
  }

  async fn delete_branch(&self, flow_id: &Uuid, recursive: bool) -> Result<()> {
    // Verificar existencia
    let _branch_metadata = {
      let flows = self.handle_poison(self.flows.lock())?;
      flows.get(flow_id).cloned().ok_or_else(|| FlowError::NotFound(format!("Branch {}", flow_id)))?
    };

    // Eliminar subramas si es recursivo
    if recursive {
      let child_ids = self.list_child_branches(flow_id).await?;
      for child_id in child_ids {
        self.delete_branch(&child_id, true).await?;
      }
    }

    // Eliminar metadata, pasos y snapshots
    {
      let mut flows = self.handle_poison(self.flows.lock())?;
      let mut steps = self.handle_poison(self.steps.lock())?;
      let mut snapshots = self.handle_poison(self.snapshots.lock())?;

      flows.remove(flow_id);
      steps.remove(flow_id);

      // Eliminar snapshots relacionados
      let snapshot_keys: Vec<Uuid> = snapshots.iter().filter(|(_, s)| s.flow_id == *flow_id).map(|(k, _)| *k).collect();

      for k in snapshot_keys {
        snapshots.remove(&k);
      }
    }

    self.clear_metadata_entries(flow_id)?;

    Ok(())
  }

  async fn list_child_branches(&self, parent_flow_id: &Uuid) -> Result<Vec<Uuid>> {
    let flows = self.handle_poison(self.flows.lock())?;
    Ok(flows.values().filter_map(|fm| if fm.parent_flow_id == Some(*parent_flow_id) { Some(fm.id) } else { None }).collect())
  }

  async fn get_branch_info(&self, flow_id: &Uuid) -> Result<Option<BranchInfo>> {
    let flows = self.handle_poison(self.flows.lock())?;
    if let Some(metadata) = flows.get(flow_id) {
      Ok(Some(BranchInfo { flow_id: *flow_id,
                           parent_flow_id: metadata.parent_flow_id,
                           parent_cursor: metadata.parent_cursor,
                           created_at: metadata.created_at,
                           metadata: metadata.metadata.clone() }))
    } else {
      Ok(None)
    }
  }

  async fn branch_exists(&self, flow_id: &Uuid) -> Result<bool> {
    self.flow_exists(flow_id).await
  }
}

// Implementación del puerto de snapshots
#[async_trait]
impl SnapshotPort for InMemoryFlowRepository {
  async fn save_snapshot(&self, flow_id: &Uuid, cursor: i64, state_ptr: &str, metadata: Value) -> Result<Uuid> {
    let snapshot_id = Uuid::new_v4();

    // Build SnapshotMeta using the same id as the map key so callers
    // that compare ids match expectations.
    let snapshot_meta = SnapshotMeta { id: snapshot_id,
                                       flow_id: *flow_id,
                                       cursor,
                                       state_ptr: state_ptr.to_string(),
                                       metadata,
                                       created_at: Utc::now() };

    // Guardar metadata
    {
      let mut snapshots = self.handle_poison(self.snapshots.lock())?;
      snapshots.insert(snapshot_id, snapshot_meta);
    }

    Ok(snapshot_id)
  }

  async fn load_latest_snapshot(&self, flow_id: &Uuid) -> Result<Option<SnapshotMeta>> {
    let snapshots = self.handle_poison(self.snapshots.lock())?;
    Ok(snapshots.values().filter(|s| s.flow_id == *flow_id).max_by_key(|s| s.cursor).cloned())
  }

  async fn load_snapshot(&self, snapshot_id: &Uuid) -> Result<(Vec<u8>, SnapshotMeta)> {
    let snapshots = self.handle_poison(self.snapshots.lock())?;
    let meta = snapshots.get(snapshot_id).cloned().ok_or_else(|| FlowError::NotFound("Snapshot not found".to_string()))?;

    let blobs = self.handle_poison(self.blobs.lock())?;
    let data = blobs.get(&meta.state_ptr).cloned().unwrap_or_default();

    Ok((data, meta))
  }

  async fn list_snapshots(&self, flow_id: &Uuid) -> Result<Vec<SnapshotMeta>> {
    let snapshots = self.handle_poison(self.snapshots.lock())?;
    Ok(snapshots.values().filter(|s| s.flow_id == *flow_id).cloned().collect())
  }

  async fn cleanup_old_snapshots(&self, flow_id: &Uuid, keep_latest: usize) -> Result<()> {
    let mut snapshots = self.handle_poison(self.snapshots.lock())?;
    let mut blobs = self.handle_poison(self.blobs.lock())?;

    // Obtener snapshots del flujo ordenados por cursor
    let mut flow_snapshots: Vec<_> =
      snapshots.iter().filter(|(_, s)| s.flow_id == *flow_id).map(|(id, meta)| (*id, meta.clone())).collect();

    flow_snapshots.sort_by_key(|(_, meta)| meta.cursor);

    // Eliminar los más antiguos
    if flow_snapshots.len() > keep_latest {
      let to_delete = &flow_snapshots[..flow_snapshots.len() - keep_latest];
      for (snapshot_id, meta) in to_delete {
        snapshots.remove(snapshot_id);
        blobs.remove(&meta.state_ptr);
      }
    }

    Ok(())
  }
}

// Implementación del puerto de blob storage
use futures::future::BoxFuture;
use futures::FutureExt;

impl BlobStorage for InMemoryFlowRepository {
  fn store_blob<'a>(&'a self, data: &'a [u8]) -> BoxFuture<'a, Result<String>> {
    async move {
      let key = format!("blob_{}", Uuid::new_v4());
      let mut blobs = self.handle_poison(self.blobs.lock())?;
      blobs.insert(key.clone(), data.to_vec());
      Ok(key)
    }.boxed()
  }

  fn retrieve_blob<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>>> {
    async move {
      let blobs = self.handle_poison(self.blobs.lock())?;
      blobs.get(key).cloned().ok_or_else(|| FlowError::NotFound(format!("Blob {}", key)))
    }.boxed()
  }

  fn delete_blob<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>> {
    async move {
      let mut blobs = self.handle_poison(self.blobs.lock())?;
      blobs.remove(key);
      Ok(())
    }.boxed()
  }

  fn blob_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool>> {
    async move {
      let blobs = self.handle_poison(self.blobs.lock())?;
      Ok(blobs.contains_key(key))
    }.boxed()
  }

  fn copy_blob<'a>(&'a self, src_key: &'a str) -> BoxFuture<'a, Result<String>> {
    async move {
      // Clone data while holding the lock to avoid keeping the guard across await
      let data = {
        let blobs = self.handle_poison(self.blobs.lock())?;
        blobs.get(src_key).cloned().ok_or_else(|| FlowError::NotFound(format!("Source blob {}", src_key)))?
      };

      // Store cloned data
      self.store_blob(&data).await
    }.boxed()
  }
}

// Implementación del puerto de metadatos
#[async_trait]
impl MetadataPort for InMemoryFlowRepository {
  async fn get_metadata(&self, flow_id: &Uuid, key: &str) -> Result<Value> {
    let metadata_store = self.handle_poison(self.metadata_store.lock())?;
    Ok(metadata_store.get(&(*flow_id, key.to_string())).cloned().unwrap_or(Value::Null))
  }

  async fn set_metadata(&self, flow_id: &Uuid, key: &str, value: Value) -> Result<()> {
    let mut metadata_store = self.handle_poison(self.metadata_store.lock())?;
    metadata_store.insert((*flow_id, key.to_string()), value);
    Ok(())
  }

  async fn delete_metadata(&self, flow_id: &Uuid, key: &str) -> Result<()> {
    let mut metadata_store = self.handle_poison(self.metadata_store.lock())?;
    metadata_store.remove(&(*flow_id, key.to_string()));
    Ok(())
  }

  async fn list_metadata_keys(&self, flow_id: &Uuid) -> Result<Vec<String>> {
    let metadata_store = self.handle_poison(self.metadata_store.lock())?;
    Ok(metadata_store.keys().filter_map(|(id, key)| if id == flow_id { Some(key.clone()) } else { None }).collect())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[tokio::test]
  async fn test_flow_metadata_operations() {
    let repo = InMemoryFlowRepository::new();

    let metadata = FlowMetadata::new(Some("test_flow"),
                                     Some("active"),
                                     Some("test_user"),
                                     None::<uuid::Uuid>,
                                     None::<i64>,
                                     json!({}));

    let flow_id = metadata.id;

    // Crear flujo
    repo.create_flow(metadata.clone()).await.unwrap();

    // Verificar existencia
    assert!(repo.flow_exists(&flow_id).await.unwrap());

    // Obtener metadata
    let retrieved = repo.get_flow_metadata(&flow_id).await.unwrap();
    assert_eq!(retrieved.name, Some("test_flow".to_string()));

    // Actualizar status
    repo.set_flow_status(&flow_id, Some("completed".to_string())).await.unwrap();
    let status = repo.get_flow_status(&flow_id).await.unwrap();
    assert_eq!(status, Some("completed".to_string()));
  }

  #[tokio::test]
  async fn test_flow_data_operations() {
    let repo = InMemoryFlowRepository::new();

    // Crear flujo primero
    let metadata = FlowMetadata::new(Some("test_flow"),
                                     Some("active"),
                                     None::<String>,
                                     None::<uuid::Uuid>,
                                     None::<i64>,
                                     json!({}));
    let flow_id = metadata.id;
    repo.create_flow(metadata).await.unwrap();

    // Crear datos
    let data = FlowData::new(flow_id, 1, "test_key", json!({"content": "test"}), json!({}), None);

    // Persistir
    let result = repo.persist_data(&data, 0).await.unwrap();
    assert!(matches!(result, PersistResult::Ok { new_version: 1 }));

    // Leer datos
    let read_data = repo.read_data(&flow_id, 0).await.unwrap();
    assert_eq!(read_data.len(), 1);
    assert_eq!(read_data[0].key, "test_key");

    // Contar datos
    let count = repo.count_flow_data(&flow_id).await.unwrap();
    assert_eq!(count, 1);
  }

  #[tokio::test]
  async fn test_branch_operations() {
    let repo = InMemoryFlowRepository::new();

    // Crear flujo principal
    let main_metadata = FlowMetadata::new(Some("main_flow"),
                                          Some("active"),
                                          None::<String>,
                                          None::<uuid::Uuid>,
                                          None::<i64>,
                                          json!({}));
    let main_flow_id = main_metadata.id;
    repo.create_flow(main_metadata).await.unwrap();

    // Añadir algunos pasos
    for i in 1..=5 {
      let data = FlowData::new(main_flow_id,
                               i,
                               format!("step_{}", i),
                               json!({"content": format!("Step {}", i)}),
                               json!({}),
                               None);
      repo.persist_data(&data, i - 1).await.unwrap();
    }

    // Crear rama
    let branch_id = repo.create_branch(&main_flow_id, 3, json!({"purpose": "testing"})).await.unwrap();

    // Verificar rama
    assert!(repo.branch_exists(&branch_id).await.unwrap());

    // Verificar relación padre-hijo
    let children = repo.list_child_branches(&main_flow_id).await.unwrap();
    assert!(children.contains(&branch_id));

    // Verificar info de rama
    let branch_info = repo.get_branch_info(&branch_id).await.unwrap().unwrap();
    assert_eq!(branch_info.parent_flow_id, Some(main_flow_id));
    assert_eq!(branch_info.parent_cursor, Some(3));
  }

  #[tokio::test]
  async fn test_snapshot_operations() {
    let repo = InMemoryFlowRepository::new();

    // Crear flujo
    let metadata = FlowMetadata::new(Some("test_flow"),
                                     Some("active"),
                                     None::<String>,
                                     None::<uuid::Uuid>,
                                     None::<i64>,
                                     json!({}));
    let flow_id = metadata.id;
    repo.create_flow(metadata).await.unwrap();

    // Crear snapshot: primero guardar bytes en el blob storage y pasar la key
    let state_data = b"test snapshot data";
    let blob_key = repo.store_blob(state_data.as_ref()).await.unwrap();
    let snapshot_id = repo.save_snapshot(&flow_id, 5, &blob_key, json!({"description": "test snapshot"})).await.unwrap();

    // Cargar snapshot
    let (loaded_data, loaded_meta) = repo.load_snapshot(&snapshot_id).await.unwrap();
    assert_eq!(loaded_data, state_data);
    assert_eq!(loaded_meta.cursor, 5);

    // Verificar último snapshot
    let latest = repo.load_latest_snapshot(&flow_id).await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().id, snapshot_id);
  }

  #[tokio::test]
  async fn test_metadata_operations() {
    let repo = InMemoryFlowRepository::new();

    // Crear flujo
    let metadata = FlowMetadata::new(Some("test_flow"),
                                     Some("active"),
                                     None::<String>,
                                     None::<uuid::Uuid>,
                                     None::<i64>,
                                     json!({}));
    let flow_id = metadata.id;
    repo.create_flow(metadata).await.unwrap();

    // Set metadata
    repo.set_metadata(&flow_id, "test_key", json!("test_value")).await.unwrap();

    // Get metadata
    let value = repo.get_metadata(&flow_id, "test_key").await.unwrap();
    assert_eq!(value, json!("test_value"));

    // List keys
    let keys = repo.list_metadata_keys(&flow_id).await.unwrap();
    assert!(keys.contains(&"test_key".to_string()));

    // Delete metadata
    repo.delete_metadata(&flow_id, "test_key").await.unwrap();
    let value = repo.get_metadata(&flow_id, "test_key").await.unwrap();
    assert_eq!(value, Value::Null);
  }
}
