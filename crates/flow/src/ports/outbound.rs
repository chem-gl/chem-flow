//! Puertos de salida - Interfaces para persistencia y almacenamiento
//!
//! Define las abstracciones que el dominio necesita para persistir datos
//! y gestionar snapshots. Estas interfaces son implementadas por los
//! adaptadores.

use crate::domain::value_objects::*;
use crate::errors::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use serde_json::Value;
use uuid::Uuid;

/// Puerto principal para repositorio de flujos
///
/// Define todas las operaciones de persistencia necesarias para el dominio.
/// Segregado en múltiples traits especializados siguiendo ISP.
#[async_trait]
pub trait FlowRepository: FlowMetadataPort + FlowDataPort + BranchManagementPort + SnapshotPort + Send + Sync {
  /// Operaciones transaccionales que requieren múltiples puertos
  async fn create_flow_with_initial_data(&self, metadata: FlowMetadata, initial_data: FlowData) -> Result<Uuid>;

  /// Obtiene estadísticas generales del repositorio
  async fn get_repository_stats(&self) -> Result<RepositoryStats>;
}

/// Puerto para gestión de metadatos de flujos
#[async_trait]
pub trait FlowMetadataPort: Send + Sync {
  /// Obtiene metadatos de un flujo
  async fn get_flow_metadata(&self, flow_id: &Uuid) -> Result<FlowMetadata>;

  /// Crea un nuevo flujo con metadatos
  async fn create_flow(&self, metadata: FlowMetadata) -> Result<Uuid>;

  /// Actualiza metadatos de un flujo existente
  async fn update_flow_metadata(&self, flow_id: &Uuid, metadata: FlowMetadata) -> Result<()>;

  /// Elimina un flujo completamente
  async fn delete_flow(&self, flow_id: &Uuid) -> Result<()>;

  /// Lista todos los IDs de flujos
  async fn list_flow_ids(&self) -> Result<Vec<Uuid>>;

  /// Verifica si un flujo existe
  async fn flow_exists(&self, flow_id: &Uuid) -> Result<bool>;

  /// Obtiene el estado actual de un flujo
  async fn get_flow_status(&self, flow_id: &Uuid) -> Result<Option<String>>;

  /// Actualiza el estado de un flujo
  async fn set_flow_status(&self, flow_id: &Uuid, status: Option<String>) -> Result<()>;
}

/// Puerto para gestión de datos de flujos (eventos)
#[async_trait]
pub trait FlowDataPort: Send + Sync {
  /// Persiste un nuevo evento con control de concurrencia optimista
  async fn persist_data(&self, data: &FlowData, expected_version: i64) -> Result<PersistResult>;

  /// Lee eventos desde un cursor específico
  async fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> Result<Vec<FlowData>>;

  /// Lee un evento específico por cursor
  async fn read_data_at_cursor(&self, flow_id: &Uuid, cursor: i64) -> Result<Option<FlowData>>;

  /// Cuenta el número total de eventos en un flujo
  async fn count_flow_data(&self, flow_id: &Uuid) -> Result<i64>;

  /// Elimina eventos desde un cursor específico
  async fn delete_data_from_cursor(&self, flow_id: &Uuid, from_cursor: i64) -> Result<()>;

  /// Verifica si existe contenido duplicado
  async fn content_exists(&self, content_hash: &str) -> Result<bool>;
}

/// Puerto para gestión de ramas
#[async_trait]
pub trait BranchManagementPort: Send + Sync {
  /// Crea una nueva rama desde un punto específico
  async fn create_branch(&self, parent_flow_id: &Uuid, parent_cursor: i64, metadata: Value) -> Result<Uuid>;

  /// Elimina una rama y opcionalmente sus subramas
  async fn delete_branch(&self, flow_id: &Uuid, recursive: bool) -> Result<()>;

  /// Lista las ramas hijas de un flujo
  async fn list_child_branches(&self, parent_flow_id: &Uuid) -> Result<Vec<Uuid>>;

  /// Obtiene información de ramificación de un flujo
  async fn get_branch_info(&self, flow_id: &Uuid) -> Result<Option<BranchInfo>>;

  /// Verifica si una rama existe
  async fn branch_exists(&self, flow_id: &Uuid) -> Result<bool>;
}

/// Puerto para gestión de snapshots
#[async_trait]
pub trait SnapshotPort: Send + Sync {
  /// Guarda un snapshot
  async fn save_snapshot(&self, flow_id: &Uuid, cursor: i64, state_ptr: &str, metadata: Value) -> Result<Uuid>;

  /// Carga el snapshot más reciente de un flujo
  async fn load_latest_snapshot(&self, flow_id: &Uuid) -> Result<Option<SnapshotMeta>>;

  /// Carga un snapshot específico por ID
  async fn load_snapshot(&self, snapshot_id: &Uuid) -> Result<(Vec<u8>, SnapshotMeta)>;

  /// Lista todos los snapshots de un flujo
  async fn list_snapshots(&self, flow_id: &Uuid) -> Result<Vec<SnapshotMeta>>;

  /// Elimina snapshots antiguos (cleanup)
  async fn cleanup_old_snapshots(&self, flow_id: &Uuid, keep_latest: usize) -> Result<()>;
}

/// Puerto para almacenamiento de objetos grandes (blobs)
/// BlobStorage: make object-safe by returning boxed futures so it can be used
/// as `dyn BlobStorage` in public APIs.
pub trait BlobStorage: Send + Sync {
  /// Almacena un blob y retorna su clave
  fn store_blob<'a>(&'a self, data: &'a [u8]) -> BoxFuture<'a, Result<String>>;

  /// Recupera un blob por su clave
  fn retrieve_blob<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Vec<u8>>>;

  /// Elimina un blob
  fn delete_blob<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<()>>;

  /// Verifica si un blob existe
  fn blob_exists<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool>>;

  /// Copia un blob (para copy-on-write)
  fn copy_blob<'a>(&'a self, src_key: &'a str) -> BoxFuture<'a, Result<String>>;
}

/// Puerto para gestión de metadatos específicos
#[async_trait]
pub trait MetadataPort: Send + Sync {
  /// Obtiene un valor de metadata por clave
  async fn get_metadata(&self, flow_id: &Uuid, key: &str) -> Result<Value>;

  /// Establece un valor de metadata
  async fn set_metadata(&self, flow_id: &Uuid, key: &str, value: Value) -> Result<()>;

  /// Elimina una clave de metadata
  async fn delete_metadata(&self, flow_id: &Uuid, key: &str) -> Result<()>;

  /// Lista todas las claves de metadata de un flujo
  async fn list_metadata_keys(&self, flow_id: &Uuid) -> Result<Vec<String>>;
}

/// Puerto para gestión de colas de trabajo
#[async_trait]
pub trait WorkQueuePort: Send + Sync {
  /// Encola un elemento de trabajo
  async fn enqueue_work(&self, work_item: WorkItem) -> Result<()>;

  /// Reclama el siguiente elemento de trabajo
  async fn claim_work(&self, worker_id: &str) -> Result<Option<WorkItem>>;

  /// Marca un trabajo como completado
  async fn complete_work(&self, work_item: &WorkItem, worker_id: &str) -> Result<()>;

  /// Marca un trabajo como fallido
  async fn fail_work(&self, work_item: &WorkItem, worker_id: &str, error: &str) -> Result<()>;

  /// Lista trabajos pendientes
  async fn list_pending_work(&self) -> Result<Vec<WorkItem>>;
}

/// Puerto para notificaciones y eventos del dominio
/// Make EventPublisher object-safe by returning boxed futures so it can be
/// passed around as `Arc<dyn EventPublisher>`.
pub trait EventPublisher: Send + Sync {
  fn publish_event<'a>(&'a self, event: DomainEvent) -> BoxFuture<'a, Result<()>>;

  fn publish_events<'a>(&'a self, events: Vec<DomainEvent>) -> BoxFuture<'a, Result<()>>;
}

// Value objects específicos para los puertos

/// Información de ramificación
#[derive(Debug, Clone, PartialEq)]
pub struct BranchInfo {
  pub flow_id: Uuid,
  pub parent_flow_id: Option<Uuid>,
  pub parent_cursor: Option<i64>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub metadata: Value,
}

/// Estadísticas del repositorio
#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryStats {
  pub total_flows: usize,
  pub total_data_records: usize,
  pub total_snapshots: usize,
  pub total_branches: usize,
  pub storage_size_bytes: u64,
}

/// Eventos de dominio
#[derive(Debug, Clone, PartialEq)]
pub enum DomainEvent {
  FlowCreated {
    flow_id: Uuid,
    name: Option<String>,
    created_by: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
  },
  StepAdded {
    flow_id: Uuid,
    cursor: i64,
    key: String,
    timestamp: chrono::DateTime<chrono::Utc>,
  },
  BranchCreated {
    branch_id: Uuid,
    parent_flow_id: Uuid,
    parent_cursor: i64,
    timestamp: chrono::DateTime<chrono::Utc>,
  },
  BranchDeleted {
    branch_id: Uuid,
    recursive: bool,
    timestamp: chrono::DateTime<chrono::Utc>,
  },
  SnapshotCreated {
    flow_id: Uuid,
    snapshot_id: Uuid,
    cursor: i64,
    timestamp: chrono::DateTime<chrono::Utc>,
  },
  FlowStatusChanged {
    flow_id: Uuid,
    old_status: Option<String>,
    new_status: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
  },
}

impl DomainEvent {
  /// Obtiene el timestamp del evento
  pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
    match self {
      DomainEvent::FlowCreated { timestamp, .. } => *timestamp,
      DomainEvent::StepAdded { timestamp, .. } => *timestamp,
      DomainEvent::BranchCreated { timestamp, .. } => *timestamp,
      DomainEvent::BranchDeleted { timestamp, .. } => *timestamp,
      DomainEvent::SnapshotCreated { timestamp, .. } => *timestamp,
      DomainEvent::FlowStatusChanged { timestamp, .. } => *timestamp,
    }
  }

  /// Obtiene el flow_id relacionado con el evento
  pub fn flow_id(&self) -> Uuid {
    match self {
      DomainEvent::FlowCreated { flow_id, .. } => *flow_id,
      DomainEvent::StepAdded { flow_id, .. } => *flow_id,
      DomainEvent::BranchCreated { parent_flow_id, .. } => *parent_flow_id,
      DomainEvent::BranchDeleted { branch_id, .. } => *branch_id, // Nota: esto es el branch_id, no flow_id
      DomainEvent::SnapshotCreated { flow_id, .. } => *flow_id,
      DomainEvent::FlowStatusChanged { flow_id, .. } => *flow_id,
    }
  }

  /// Obtiene el tipo de evento como string
  pub fn event_type(&self) -> &'static str {
    match self {
      DomainEvent::FlowCreated { .. } => "FlowCreated",
      DomainEvent::StepAdded { .. } => "StepAdded",
      DomainEvent::BranchCreated { .. } => "BranchCreated",
      DomainEvent::BranchDeleted { .. } => "BranchDeleted",
      DomainEvent::SnapshotCreated { .. } => "SnapshotCreated",
      DomainEvent::FlowStatusChanged { .. } => "FlowStatusChanged",
    }
  }
}
