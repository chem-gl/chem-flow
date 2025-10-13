//! Puertos de entrada - Interfaces para casos de uso y comandos
//!
//! Define las abstracciones que expone el dominio hacia el exterior.
//! Estas interfaces son implementadas por los servicios de aplicación.

use crate::domain::value_objects::*;
use crate::errors::Result;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// Puerto principal para gestión de flujos
///
/// Define las operaciones de alto nivel que puede realizar un usuario
/// del sistema de flujos.
#[async_trait]
pub trait FlowManagementService: Send + Sync {
  /// Crea un nuevo flujo con datos iniciales
  async fn create_flow(&self,
                       name: Option<String>,
                       status: Option<String>,
                       metadata: Value,
                       initial_step: Option<CreateStepRequest>)
                       -> Result<FlowCreationResult>;

  /// Añade un paso a una rama específica
  async fn add_step(&self, flow_id: &Uuid, request: AddStepRequest) -> Result<StepCreationResult>;

  /// Crea una nueva rama desde un punto específico
  async fn create_branch(&self, request: CreateBranchRequest) -> Result<BranchCreationResult>;

  /// Elimina una rama y opcionalmente sus subramas
  async fn delete_branch(&self, request: DeleteBranchRequest) -> Result<()>;

  /// Obtiene el path completo de una rama
  async fn get_branch_path(&self, flow_id: &Uuid) -> Result<BranchPathResult>;

  /// Obtiene metadatos de un flujo
  async fn get_flow_info(&self, flow_id: &Uuid) -> Result<FlowInfoResult>;

  /// Lista todos los flujos
  async fn list_flows(&self) -> Result<Vec<FlowSummary>>;
}

/// Puerto para operaciones de snapshot
#[async_trait]
pub trait SnapshotService: Send + Sync {
  /// Crea un snapshot del estado actual de un flujo
  async fn create_snapshot(&self, request: CreateSnapshotRequest) -> Result<SnapshotCreationResult>;

  /// Rehidrata un flujo desde su snapshot más reciente
  async fn rehydrate_from_snapshot(&self, flow_id: &Uuid) -> Result<RehydrationResult>;

  /// Lista snapshots de un flujo
  async fn list_snapshots(&self, flow_id: &Uuid) -> Result<Vec<SnapshotSummary>>;

  /// Limpia snapshots antiguos
  async fn cleanup_snapshots(&self, flow_id: &Uuid, keep_latest: usize) -> Result<CleanupResult>;
}

/// Puerto para consultas y reportes
#[async_trait]
pub trait FlowQueryService: Send + Sync {
  /// Busca flujos por criterios
  async fn search_flows(&self, criteria: SearchCriteria) -> Result<SearchResult>;

  /// Obtiene estadísticas de un flujo
  async fn get_flow_statistics(&self, flow_id: &Uuid) -> Result<FlowStatistics>;

  /// Obtiene el árbol completo de ramas de un flujo
  async fn get_branch_tree(&self, flow_id: &Uuid) -> Result<BranchTree>;

  /// Valida la integridad de un flujo
  async fn validate_flow_integrity(&self, flow_id: &Uuid) -> Result<IntegrityReport>;
}

/// Puerto para gestión de metadatos
#[async_trait]
pub trait MetadataService: Send + Sync {
  /// Obtiene metadata de un flujo
  async fn get_metadata(&self, flow_id: &Uuid, key: &str) -> Result<Value>;

  /// Establece metadata de un flujo
  async fn set_metadata(&self, flow_id: &Uuid, key: &str, value: Value) -> Result<()>;

  /// Elimina metadata de un flujo
  async fn delete_metadata(&self, flow_id: &Uuid, key: &str) -> Result<()>;

  /// Lista todas las claves de metadata
  async fn list_metadata_keys(&self, flow_id: &Uuid) -> Result<Vec<String>>;

  /// Actualiza múltiples metadatos en lote
  async fn update_metadata_batch(&self, flow_id: &Uuid, updates: Vec<MetadataUpdate>) -> Result<()>;
}

/// Puerto para trabajadores y procesamiento asíncrono
#[async_trait]
pub trait WorkerService: Send + Sync {
  /// Reclama trabajo para un worker
  async fn claim_work(&self, worker_id: &str) -> Result<Option<WorkAssignment>>;

  /// Reporta progreso de trabajo
  async fn report_progress(&self, work_id: &Uuid, progress: WorkProgress) -> Result<()>;

  /// Completa un trabajo
  async fn complete_work(&self, work_id: &Uuid, result: WorkResult) -> Result<()>;

  /// Reporta fallo en trabajo
  async fn fail_work(&self, work_id: &Uuid, error: &str) -> Result<()>;
}

// DTOs y Value Objects para los puertos de entrada

/// Request para crear un nuevo paso
#[derive(Debug, Clone)]
pub struct CreateStepRequest {
  pub key: String,
  pub payload: Value,
  pub metadata: Value,
}

/// Request para añadir un paso
#[derive(Debug, Clone)]
pub struct AddStepRequest {
  pub key: String,
  pub payload: Value,
  pub metadata: Value,
  pub command_id: Option<Uuid>,
}

/// Request para crear una rama
#[derive(Debug, Clone)]
pub struct CreateBranchRequest {
  pub parent_flow_id: Uuid,
  pub parent_cursor: i64,
  pub name: Option<String>,
  pub metadata: Value,
}

/// Request para eliminar una rama
#[derive(Debug, Clone)]
pub struct DeleteBranchRequest {
  pub flow_id: Uuid,
  pub recursive: bool,
  pub reason: Option<String>,
}

/// Request para crear un snapshot
#[derive(Debug, Clone)]
pub struct CreateSnapshotRequest {
  pub flow_id: Uuid,
  pub description: Option<String>,
  pub metadata: Value,
}

/// Resultado de creación de flujo
#[derive(Debug, Clone)]
pub struct FlowCreationResult {
  pub flow_id: Uuid,
  pub name: Option<String>,
  pub initial_step_id: Option<Uuid>,
  pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Resultado de creación de paso
#[derive(Debug, Clone)]
pub struct StepCreationResult {
  pub step_id: Uuid,
  pub cursor: i64,
  pub flow_id: Uuid,
  pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Resultado de creación de rama
#[derive(Debug, Clone)]
pub struct BranchCreationResult {
  pub branch_id: Uuid,
  pub parent_flow_id: Uuid,
  pub parent_cursor: i64,
  pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Resultado de path de rama
#[derive(Debug, Clone)]
pub struct BranchPathResult {
  pub flow_id: Uuid,
  pub steps: Vec<FlowData>,
  pub total_steps: usize,
  pub branch_point: Option<i64>,
}

/// Información de flujo
#[derive(Debug, Clone)]
pub struct FlowInfoResult {
  pub metadata: FlowMetadata,
  pub step_count: i64,
  pub branch_count: usize,
  pub latest_snapshot: Option<SnapshotMeta>,
  pub child_branches: Vec<Uuid>,
}

/// Resumen de flujo
#[derive(Debug, Clone)]
pub struct FlowSummary {
  pub id: Uuid,
  pub name: Option<String>,
  pub status: Option<String>,
  pub step_count: i64,
  pub branch_count: usize,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Resultado de creación de snapshot
#[derive(Debug, Clone)]
pub struct SnapshotCreationResult {
  pub snapshot_id: Uuid,
  pub cursor: i64,
  pub size_bytes: usize,
  pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Resultado de rehidratación
#[derive(Debug, Clone)]
pub struct RehydrationResult {
  pub flow_id: Uuid,
  pub snapshot_cursor: i64,
  pub replayed_steps: usize,
  pub final_cursor: i64,
  pub rehydration_time_ms: u64,
}

/// Resumen de snapshot
#[derive(Debug, Clone)]
pub struct SnapshotSummary {
  pub id: Uuid,
  pub cursor: i64,
  pub size_bytes: usize,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub description: Option<String>,
}

/// Resultado de limpieza
#[derive(Debug, Clone)]
pub struct CleanupResult {
  pub snapshots_deleted: usize,
  pub bytes_freed: u64,
}

/// Criterios de búsqueda
#[derive(Debug, Clone)]
pub struct SearchCriteria {
  pub name_pattern: Option<String>,
  pub status: Option<String>,
  pub created_by: Option<String>,
  pub created_after: Option<chrono::DateTime<chrono::Utc>>,
  pub created_before: Option<chrono::DateTime<chrono::Utc>>,
  pub metadata_filters: Vec<MetadataFilter>,
  pub limit: Option<usize>,
  pub offset: Option<usize>,
}

/// Filtro de metadata
#[derive(Debug, Clone)]
pub struct MetadataFilter {
  pub key: String,
  pub operation: FilterOperation,
  pub value: Value,
}

/// Operaciones de filtro
#[derive(Debug, Clone)]
pub enum FilterOperation {
  Equals,
  NotEquals,
  Contains,
  GreaterThan,
  LessThan,
  Exists,
  NotExists,
}

/// Resultado de búsqueda
#[derive(Debug, Clone)]
pub struct SearchResult {
  pub flows: Vec<FlowSummary>,
  pub total_count: usize,
  pub has_more: bool,
}

/// Estadísticas de flujo
#[derive(Debug, Clone)]
pub struct FlowStatistics {
  pub flow_id: Uuid,
  pub total_steps: i64,
  pub total_branches: usize,
  pub total_snapshots: usize,
  pub storage_size_bytes: u64,
  pub creation_rate: f64, // pasos por día
  pub branch_depth: usize,
  pub content_diversity: f64, // ratio de contenido único
}

/// Árbol de ramas
#[derive(Debug, Clone)]
pub struct BranchTree {
  pub flow_id: Uuid,
  pub root_branch: BranchNode,
}

/// Nodo en el árbol de ramas
#[derive(Debug, Clone)]
pub struct BranchNode {
  pub branch_id: Uuid,
  pub name: Option<String>,
  pub step_count: i64,
  pub branch_point: Option<i64>,
  pub children: Vec<BranchNode>,
  pub metadata: Value,
}

/// Reporte de integridad
#[derive(Debug, Clone)]
pub struct IntegrityReport {
  pub flow_id: Uuid,
  pub is_valid: bool,
  pub issues: Vec<IntegrityIssue>,
  pub warnings: Vec<String>,
  pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// Problema de integridad
#[derive(Debug, Clone)]
pub struct IntegrityIssue {
  pub severity: IssueSeverity,
  pub description: String,
  pub affected_elements: Vec<String>,
  pub suggestion: Option<String>,
}

/// Severidad del problema
#[derive(Debug, Clone)]
pub enum IssueSeverity {
  Critical,
  Warning,
  Info,
}

/// Actualización de metadata
#[derive(Debug, Clone)]
pub struct MetadataUpdate {
  pub key: String,
  pub operation: MetadataOperation,
  pub value: Option<Value>,
}

/// Operación de metadata
#[derive(Debug, Clone)]
pub enum MetadataOperation {
  Set(Value),
  Delete,
  Increment(i64),
  Append(Value),
}

/// Asignación de trabajo
#[derive(Debug, Clone)]
pub struct WorkAssignment {
  pub work_id: Uuid,
  pub work_item: WorkItem,
  pub assigned_at: chrono::DateTime<chrono::Utc>,
  pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

/// Progreso de trabajo
#[derive(Debug, Clone)]
pub struct WorkProgress {
  pub percentage: f32,
  pub current_step: String,
  pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
  pub metadata: Value,
}

/// Resultado de trabajo
#[derive(Debug, Clone)]
pub struct WorkResult {
  pub status: WorkStatus,
  pub output: Value,
  pub execution_time_ms: u64,
  pub metadata: Value,
}

/// Estado de trabajo
#[derive(Debug, Clone)]
pub enum WorkStatus {
  Completed,
  PartiallyCompleted,
  Failed,
  Cancelled,
}
