//! Servicios de aplicación - Implementaciones de puertos de entrada
//!
//! Los servicios de aplicación orquestan los casos de uso y proporcionan
//! implementaciones concretas de los puertos de entrada.

use crate::application::use_cases::*;
use crate::errors::{FlowError, Result};
use crate::ports::*;
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::FutureExt;
use std::sync::Arc;
use uuid::Uuid;

/// Servicio de gestión de flujos
///
/// Implementa `FlowManagementService` orchestando los casos de uso
/// correspondientes.
pub struct FlowManagementServiceImpl<R>
  where R: FlowRepository + 'static
{
  create_flow_use_case: CreateFlowUseCase<R>,
  add_step_use_case: AddStepUseCase<R>,
  create_branch_use_case: CreateBranchUseCase<R>,
  delete_branch_use_case: DeleteBranchUseCase<R>,
  get_branch_path_use_case: GetBranchPathUseCase<R>,
  get_flow_info_use_case: GetFlowInfoUseCase<R>,
  repository: Arc<R>,
}

impl<R> FlowManagementServiceImpl<R> where R: FlowRepository + 'static
{
  /// Crea una nueva instancia del servicio
  pub fn new(repository: Arc<R>, event_publisher: Option<Arc<dyn EventPublisher>>) -> Self {
    Self { create_flow_use_case: CreateFlowUseCase::new(repository.clone(), event_publisher.clone()),
           add_step_use_case: AddStepUseCase::new(repository.clone(), event_publisher.clone()),
           create_branch_use_case: CreateBranchUseCase::new(repository.clone(), event_publisher.clone()),
           delete_branch_use_case: DeleteBranchUseCase::new(repository.clone(), event_publisher.clone()),
           get_branch_path_use_case: GetBranchPathUseCase::new(repository.clone()),
           get_flow_info_use_case: GetFlowInfoUseCase::new(repository.clone()),
           repository }
  }
}

#[async_trait]
impl<R> FlowManagementService for FlowManagementServiceImpl<R> where R: FlowRepository + 'static
{
  async fn create_flow(&self,
                       name: Option<String>,
                       status: Option<String>,
                       metadata: serde_json::Value,
                       initial_step: Option<CreateStepRequest>)
                       -> Result<FlowCreationResult> {
    self.create_flow_use_case.execute(name, status, metadata, initial_step).await
  }

  async fn add_step(&self, flow_id: &Uuid, request: AddStepRequest) -> Result<StepCreationResult> {
    self.add_step_use_case.execute(flow_id, request).await
  }

  async fn create_branch(&self, request: CreateBranchRequest) -> Result<BranchCreationResult> {
    self.create_branch_use_case.execute(request).await
  }

  async fn delete_branch(&self, request: DeleteBranchRequest) -> Result<()> {
    self.delete_branch_use_case.execute(request).await
  }

  async fn get_branch_path(&self, flow_id: &Uuid) -> Result<BranchPathResult> {
    self.get_branch_path_use_case.execute(flow_id).await
  }

  async fn get_flow_info(&self, flow_id: &Uuid) -> Result<FlowInfoResult> {
    self.get_flow_info_use_case.execute(flow_id).await
  }

  async fn list_flows(&self) -> Result<Vec<FlowSummary>> {
    let flow_ids = self.repository.list_flow_ids().await?;
    let mut summaries = Vec::new();

    for flow_id in flow_ids {
      let metadata = self.repository.get_flow_metadata(&flow_id).await?;
      let step_count = self.repository.count_flow_data(&flow_id).await?;
      let child_branches = self.repository.list_child_branches(&flow_id).await?;

      summaries.push(FlowSummary { id: flow_id,
                                   name: metadata.name,
                                   status: metadata.status,
                                   step_count,
                                   branch_count: child_branches.len(),
                                   created_at: metadata.created_at,
                                   updated_at: metadata.created_at /* En el futuro podríamos trackear updated_at */ });
    }

    Ok(summaries)
  }
}

/// Servicio de snapshots
pub struct SnapshotServiceImpl<R>
  where R: FlowRepository + 'static
{
  create_snapshot_use_case: CreateSnapshotUseCase<R>,
  rehydrate_use_case: RehydrateFromSnapshotUseCase<R>,
  repository: Arc<R>,
}

impl<R> SnapshotServiceImpl<R> where R: FlowRepository + 'static
{
  pub fn new(repository: Arc<R>,
             blob_storage: Option<Arc<dyn BlobStorage>>,
             event_publisher: Option<Arc<dyn EventPublisher>>)
             -> Self {
    Self { create_snapshot_use_case: CreateSnapshotUseCase::new(repository.clone(), blob_storage.clone(), event_publisher),
           rehydrate_use_case: RehydrateFromSnapshotUseCase::new(repository.clone(), blob_storage),
           repository }
  }
}

#[async_trait]
impl<R> SnapshotService for SnapshotServiceImpl<R> where R: FlowRepository + 'static
{
  async fn create_snapshot(&self, request: CreateSnapshotRequest) -> Result<SnapshotCreationResult> {
    self.create_snapshot_use_case.execute(request).await
  }

  async fn rehydrate_from_snapshot(&self, flow_id: &Uuid) -> Result<RehydrationResult> {
    self.rehydrate_use_case.execute(flow_id).await
  }

  async fn list_snapshots(&self, flow_id: &Uuid) -> Result<Vec<SnapshotSummary>> {
    let snapshots = self.repository.list_snapshots(flow_id).await?;

    let summaries =
      snapshots.into_iter()
               .map(|snapshot| SnapshotSummary { id: snapshot.id,
                                                 cursor: snapshot.cursor,
                                                 size_bytes: snapshot.state_ptr.len(), /* Aproximación, en
                                                                                        * implementación real se
                                                                                        * obtendría del blob storage */
                                                 created_at: snapshot.created_at,
                                                 description: snapshot.metadata
                                                                      .get("description")
                                                                      .and_then(|v| v.as_str())
                                                                      .map(|s| s.to_string()) })
               .collect();

    Ok(summaries)
  }

  async fn cleanup_snapshots(&self, flow_id: &Uuid, keep_latest: usize) -> Result<CleanupResult> {
    self.repository.cleanup_old_snapshots(flow_id, keep_latest).await?;

    // En una implementación real, aquí calcularíamos los bytes liberados
    Ok(CleanupResult { snapshots_deleted: 0, // Se calcularía basado en los snapshots eliminados
                       bytes_freed: 0 })
  }
}

/// Servicio de consultas
pub struct FlowQueryServiceImpl<R>
  where R: FlowRepository + 'static
{
  repository: Arc<R>,
}

impl<R> FlowQueryServiceImpl<R> where R: FlowRepository + 'static
{
  pub fn new(repository: Arc<R>) -> Self {
    Self { repository }
  }

  /// Construye el árbol de ramas recursivamente
  fn build_branch_tree_recursive<'a>(&'a self,
                                     flow_id: &'a Uuid,
                                     visited: &'a mut std::collections::HashSet<Uuid>)
                                     -> BoxFuture<'a, Result<BranchNode>> {
    async move {
      if visited.contains(flow_id) {
        return Err(FlowError::Other("Circular reference detected".to_string()));
      }
      visited.insert(*flow_id);

      let metadata = self.repository.get_flow_metadata(flow_id).await?;
      let step_count = self.repository.count_flow_data(flow_id).await?;
      let child_branches = self.repository.list_child_branches(flow_id).await?;

      let mut children = Vec::new();
      for child_id in child_branches {
        let child_node = self.build_branch_tree_recursive(&child_id, visited).await?;
        children.push(child_node);
      }

      Ok(BranchNode { branch_id: *flow_id,
                      name: metadata.name,
                      step_count,
                      branch_point: metadata.parent_cursor,
                      children,
                      metadata: metadata.metadata })
    }.boxed()
  }
}

#[async_trait]
impl<R> FlowQueryService for FlowQueryServiceImpl<R> where R: FlowRepository + 'static
{
  async fn search_flows(&self, criteria: SearchCriteria) -> Result<SearchResult> {
    // Implementación simplificada - en un caso real usaríamos filtros más
    // sofisticados
    let all_flows = self.repository.list_flow_ids().await?;
    let mut matching_flows = Vec::new();

    for flow_id in all_flows {
      let metadata = self.repository.get_flow_metadata(&flow_id).await?;

      // Aplicar filtros básicos
      let mut matches = true;

      if let Some(ref name_pattern) = criteria.name_pattern {
        if let Some(ref flow_name) = metadata.name {
          if !flow_name.contains(name_pattern) {
            matches = false;
          }
        } else {
          matches = false;
        }
      }

      if let Some(ref status) = criteria.status {
        if metadata.status.as_ref() != Some(status) {
          matches = false;
        }
      }

      if let Some(created_after) = criteria.created_after {
        if metadata.created_at <= created_after {
          matches = false;
        }
      }

      if let Some(created_before) = criteria.created_before {
        if metadata.created_at >= created_before {
          matches = false;
        }
      }

      if matches {
        let step_count = self.repository.count_flow_data(&flow_id).await?;
        let child_branches = self.repository.list_child_branches(&flow_id).await?;

        matching_flows.push(FlowSummary { id: flow_id,
                                          name: metadata.name,
                                          status: metadata.status,
                                          step_count,
                                          branch_count: child_branches.len(),
                                          created_at: metadata.created_at,
                                          updated_at: metadata.created_at });
      }

      // Aplicar límite si está especificado
      if let Some(limit) = criteria.limit {
        if matching_flows.len() >= limit {
          break;
        }
      }
    }

    let total_count = matching_flows.len();
    let has_more = criteria.limit.is_some_and(|limit| total_count >= limit);

    Ok(SearchResult { flows: matching_flows, total_count, has_more })
  }

  async fn get_flow_statistics(&self, flow_id: &Uuid) -> Result<FlowStatistics> {
    let metadata = self.repository.get_flow_metadata(flow_id).await?;
    let step_count = self.repository.count_flow_data(flow_id).await?;
    let child_branches = self.repository.list_child_branches(flow_id).await?;
    let snapshots = self.repository.list_snapshots(flow_id).await?;

    // Calcular tasa de creación (simplificado)
    let days_since_creation = (chrono::Utc::now() - metadata.created_at).num_days() as f64;
    let creation_rate = if days_since_creation > 0.0 { step_count as f64 / days_since_creation } else { step_count as f64 };

    // Calcular profundidad de ramas (simplificado)
    let branch_depth = self.calculate_max_branch_depth(flow_id, 0).await?;

    Ok(FlowStatistics { flow_id: *flow_id,
                        total_steps: step_count,
                        total_branches: child_branches.len(),
                        total_snapshots: snapshots.len(),
                        storage_size_bytes: 0, // Se calcularía en implementación real
                        creation_rate,
                        branch_depth,
                        content_diversity: 1.0 /* Se calcularía analizando diversidad de contenido */ })
  }

  async fn get_branch_tree(&self, flow_id: &Uuid) -> Result<BranchTree> {
    let mut visited = std::collections::HashSet::new();
    let root_branch = self.build_branch_tree_recursive(flow_id, &mut visited).await?;

    Ok(BranchTree { flow_id: *flow_id, root_branch })
  }

  async fn validate_flow_integrity(&self, flow_id: &Uuid) -> Result<IntegrityReport> {
    let mut issues = Vec::new();
    let warnings = Vec::new();

    // Verificar que el flujo existe
    if !self.repository.flow_exists(flow_id).await? {
      issues.push(IntegrityIssue { severity: IssueSeverity::Critical,
                                   description: "Flow does not exist".to_string(),
                                   affected_elements: vec![flow_id.to_string()],
                                   suggestion: Some("Create the flow or check the ID".to_string()) });
    } else {
      // Verificar metadatos
      let metadata = self.repository.get_flow_metadata(flow_id).await?;
      let step_count = self.repository.count_flow_data(flow_id).await?;

      if metadata.current_cursor != step_count {
        issues.push(IntegrityIssue { severity: IssueSeverity::Warning,
                                     description: "Cursor mismatch with actual step count".to_string(),
                                     affected_elements: vec![flow_id.to_string()],
                                     suggestion: Some("Rebuild metadata from events".to_string()) });
      }

      // Verificar ramas hijas
      let child_branches = self.repository.list_child_branches(flow_id).await?;
      for child_id in child_branches {
        if !self.repository.branch_exists(&child_id).await? {
          issues.push(IntegrityIssue { severity: IssueSeverity::Critical,
                                       description: "Referenced child branch does not exist".to_string(),
                                       affected_elements: vec![child_id.to_string()],
                                       suggestion: Some("Remove invalid reference or restore branch".to_string()) });
        }
      }
    }

    let is_valid = issues.iter().all(|i| matches!(i.severity, IssueSeverity::Warning | IssueSeverity::Info));

    Ok(IntegrityReport { flow_id: *flow_id, is_valid, issues, warnings, checked_at: chrono::Utc::now() })
  }
}

impl<R> FlowQueryServiceImpl<R> where R: FlowRepository + 'static
{
  fn calculate_max_branch_depth<'a>(&'a self, flow_id: &'a Uuid, current_depth: usize) -> BoxFuture<'a, Result<usize>> {
    async move {
      let child_branches = self.repository.list_child_branches(flow_id).await?;

      if child_branches.is_empty() {
        return Ok(current_depth);
      }

      let mut max_depth = current_depth;
      for child_id in child_branches {
        let child_depth = self.calculate_max_branch_depth(&child_id, current_depth + 1).await?;
        max_depth = max_depth.max(child_depth);
      }

      Ok(max_depth)
    }.boxed()
  }
}

/// Servicio de metadatos
pub struct MetadataServiceImpl<R>
  where R: crate::ports::outbound::FlowRepository + crate::ports::outbound::MetadataPort + 'static
{
  repository: Arc<R>,
}

impl<R> MetadataServiceImpl<R>
  where R: crate::ports::outbound::FlowRepository + crate::ports::outbound::MetadataPort + 'static
{
  pub fn new(repository: Arc<R>) -> Self {
    Self { repository }
  }
}

#[async_trait]
impl<R> MetadataService for MetadataServiceImpl<R>
  where R: crate::ports::outbound::FlowRepository + crate::ports::outbound::MetadataPort + 'static
{
  async fn get_metadata(&self, flow_id: &Uuid, key: &str) -> Result<serde_json::Value> {
    self.repository.get_metadata(flow_id, key).await
  }

  async fn set_metadata(&self, flow_id: &Uuid, key: &str, value: serde_json::Value) -> Result<()> {
    self.repository.set_metadata(flow_id, key, value).await
  }

  async fn delete_metadata(&self, flow_id: &Uuid, key: &str) -> Result<()> {
    self.repository.delete_metadata(flow_id, key).await
  }

  async fn list_metadata_keys(&self, flow_id: &Uuid) -> Result<Vec<String>> {
    self.repository.list_metadata_keys(flow_id).await
  }

  async fn update_metadata_batch(&self, flow_id: &Uuid, updates: Vec<MetadataUpdate>) -> Result<()> {
    for update in updates {
      match update.operation {
        MetadataOperation::Set(value) => {
          self.repository.set_metadata(flow_id, &update.key, value).await?;
        }
        MetadataOperation::Delete => {
          self.repository.delete_metadata(flow_id, &update.key).await?;
        }
        MetadataOperation::Increment(amount) => {
          let current = self.repository.get_metadata(flow_id, &update.key).await?;
          let new_value = if let Some(num) = current.as_i64() {
            serde_json::Value::Number((num + amount).into())
          } else {
            serde_json::Value::Number(amount.into())
          };
          self.repository.set_metadata(flow_id, &update.key, new_value).await?;
        }
        MetadataOperation::Append(value) => {
          let current = self.repository.get_metadata(flow_id, &update.key).await?;
          let new_value = if let Some(array) = current.as_array() {
            let mut new_array = array.clone();
            new_array.push(value);
            serde_json::Value::Array(new_array)
          } else {
            serde_json::Value::Array(vec![value])
          };
          self.repository.set_metadata(flow_id, &update.key, new_value).await?;
        }
      }
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::adapters::InMemoryFlowRepository;
  use serde_json::json;

  #[tokio::test]
  async fn test_flow_management_service() {
    let repo = Arc::new(InMemoryFlowRepository::new());
    let service = FlowManagementServiceImpl::new(repo, None);

    // Crear flujo
    let result = service.create_flow(Some("Test Flow".to_string()), Some("active".to_string()), json!({}), None).await;

    assert!(result.is_ok());
    let flow = result.unwrap();

    // Añadir paso
    let add_request = AddStepRequest { key: "test_step".to_string(),
                                       payload: json!({"content": "test"}),
                                       metadata: json!({}),
                                       command_id: None };

    let step_result = service.add_step(&flow.flow_id, add_request).await;
    assert!(step_result.is_ok());

    // Obtener información del flujo
    let info_result = service.get_flow_info(&flow.flow_id).await;
    assert!(info_result.is_ok());

    let info = info_result.unwrap();
    assert_eq!(info.step_count, 1);
  }

  #[tokio::test]
  async fn test_branch_operations() {
    let repo = Arc::new(InMemoryFlowRepository::new());
    let service = FlowManagementServiceImpl::new(repo, None);

    // Crear flujo principal
    let flow =
      service.create_flow(Some("Main Flow".to_string()), Some("active".to_string()), json!({}), None).await.unwrap();

    // Añadir algunos pasos
    for i in 1..=5 {
      let add_request = AddStepRequest { key: format!("step_{}", i),
                                         payload: json!({"content": format!("Step {}", i)}),
                                         metadata: json!({}),
                                         command_id: None };
      service.add_step(&flow.flow_id, add_request).await.unwrap();
    }

    // Crear rama
    let branch_request = CreateBranchRequest { parent_flow_id: flow.flow_id,
                                               parent_cursor: 3,
                                               name: Some("Test Branch".to_string()),
                                               metadata: json!({"purpose": "testing"}) };

    let branch_result = service.create_branch(branch_request).await;
    assert!(branch_result.is_ok());

    let branch = branch_result.unwrap();
    assert_eq!(branch.parent_flow_id, flow.flow_id);
    assert_eq!(branch.parent_cursor, 3);
  }
}
