//! # Flow - Sistema de Árbol de Flujos
//!
//! Biblioteca para gestión de flujos basada en Event Sourcing que implementa
//! una estructura de árbol dirigido acíclico (DAG) similar a un sistema de
//! control de versiones simplificado.
//!
//! ## Arquitectura
//!
//! Sigue principios de arquitectura hexagonal (puertos y adaptadores):
//!
//! - **Dominio**: Lógica de negocio central y entidades
//! - **Puertos**: Interfaces que definen contratos
//! - **Adaptadores**: Implementaciones concretas de infraestructura
//! - **Aplicación**: Casos de uso y servicios de aplicación
//!
//! ## Características Principales
//!
//! - **Sin Ciclos**: Estructura de árbol que previene bucles
//! - **Sin Merges**: No permite fusión de ramas divergentes
//! - **Sin Duplicaciones**: Verificación global de unicidad de contenido
//! - **Eliminación Recursiva**: Borrado automático de subramas dependientes
//! - **Event Sourcing**: Estado inmutable reconstituido desde eventos
//! - **Snapshots**: Optimización de rendimiento mediante checkpoints
//!
//! ## Ejemplo de Uso
//!
//! ```rust
//! use flow::*;
//! // Import concrete service and request types used in this example
//! # use flow::application::FlowManagementServiceImpl;
//! # use flow::ports::{AddStepRequest, CreateBranchRequest, FlowManagementService};
//! use std::sync::Arc;
//! use serde_json::json;
//!
//! # async fn example() -> std::result::Result<(), Box<dyn std::error::Error>> {
//! // Configurar repositorio
//! let repository = Arc::new(InMemoryFlowRepository::new());
//!
//! // Crear servicio de gestión
//! let flow_service = FlowManagementServiceImpl::new(repository, None);
//!
//! // Crear un nuevo flujo
//! let flow_result = flow_service.create_flow(
//!     Some("Experimento Químico".to_string()),
//!     Some("active".to_string()),
//!     json!({"experiment_type": "synthesis"}),
//!     None
//! ).await?;
//!
//! // Añadir pasos al flujo
//! let step_request = AddStepRequest {
//!     key: "preparation".to_string(),
//!     payload: json!({"materials": ["reactivo_a", "reactivo_b"]}),
//!     metadata: json!({"duration_min": 30}),
//!     command_id: None,
//! };
//!
//! let step_result = flow_service.add_step(&flow_result.flow_id, step_request).await?;
//!
//! // Crear rama para explorar alternativa
//! let branch_request = CreateBranchRequest {
//!     parent_flow_id: flow_result.flow_id,
//!     parent_cursor: 1,
//!     name: Some("Temperatura alternativa".to_string()),
//!     metadata: json!({"hypothesis": "mayor temperatura mejora rendimiento"}),
//! };
//!
//! let branch_result = flow_service.create_branch(branch_request).await?;
//! # Ok(())
//! # }
//! ```

// Módulos principales de la arquitectura hexagonal
pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

// Legacy/infra modules
pub mod engine;
pub mod errors;
pub mod stubs;

pub use adapters::InMemoryFlowRepository;

// Errors
pub use errors::*;

// The crate historically had a `repository` module (src/repository.rs)
// which defined the synchronous testing/stub traits (SnapshotStore,
// ArtifactStore, and a legacy FlowRepository). During the async refactor
// a compatibility inline module was introduced which shadowed the file
// module and caused unresolved imports in `stubs.rs` and tests.
//
// Restore the original file-based `repository` module so legacy stubs
// and tests continue to compile. For the new async outbound trait we
// expose it under `repository_async` to avoid name collisions while
// migration proceeds.
pub mod repository;

// Async outbound FlowRepository (new ports) exposed under a distinct
// module name to avoid colliding with the legacy sync `repository`.
pub mod repository_async {
  pub use crate::ports::outbound::FlowRepository;
}

// Facade principal para configuración rápida
use std::sync::Arc;

use crate::application::{FlowManagementServiceImpl, FlowQueryServiceImpl, MetadataServiceImpl, SnapshotServiceImpl};

/// Facade principal para configurar el sistema completo
///
/// Proporciona una interfaz simple para configurar todos los servicios
/// necesarios con configuración por defecto.
pub struct FlowSystem<R>
  where R: crate::ports::outbound::FlowRepository + crate::ports::outbound::MetadataPort + 'static
{
  pub flow_service: FlowManagementServiceImpl<R>,
  pub snapshot_service: SnapshotServiceImpl<R>,
  pub query_service: FlowQueryServiceImpl<R>,
  pub metadata_service: MetadataServiceImpl<R>,
  pub repository: Arc<R>,
}

impl<R> FlowSystem<R> where R: crate::ports::outbound::FlowRepository + crate::ports::outbound::MetadataPort + 'static
{
  /// Crea una nueva instancia del sistema con el repositorio especificado
  pub fn new(repository: Arc<R>) -> Self {
    Self { flow_service: FlowManagementServiceImpl::new(repository.clone(), None),
           snapshot_service: SnapshotServiceImpl::new(repository.clone(), None, None),
           query_service: FlowQueryServiceImpl::new(repository.clone()),
           metadata_service: MetadataServiceImpl::new(repository.clone()),
           repository }
  }

  /// Crea una nueva instancia con publisher de eventos
  pub fn with_event_publisher(repository: Arc<R>, event_publisher: Arc<dyn crate::ports::outbound::EventPublisher>) -> Self {
    Self { flow_service: FlowManagementServiceImpl::new(repository.clone(), Some(event_publisher.clone())),
           snapshot_service: SnapshotServiceImpl::new(repository.clone(), None, Some(event_publisher)),
           query_service: FlowQueryServiceImpl::new(repository.clone()),
           metadata_service: MetadataServiceImpl::new(repository.clone()),
           repository }
  }

  /// Crea una nueva instancia con blob storage para snapshots
  pub fn with_blob_storage(repository: Arc<R>, blob_storage: Arc<dyn crate::ports::outbound::BlobStorage>) -> Self {
    Self { flow_service: FlowManagementServiceImpl::new(repository.clone(), None),
           snapshot_service: SnapshotServiceImpl::new(repository.clone(), Some(blob_storage), None),
           query_service: FlowQueryServiceImpl::new(repository.clone()),
           metadata_service: MetadataServiceImpl::new(repository.clone()),
           repository }
  }

  /// Configuración completa con todas las opciones
  pub fn with_full_config(repository: Arc<R>,
                          blob_storage: Option<Arc<dyn crate::ports::outbound::BlobStorage>>,
                          event_publisher: Option<Arc<dyn crate::ports::outbound::EventPublisher>>)
                          -> Self {
    Self { flow_service: FlowManagementServiceImpl::new(repository.clone(), event_publisher.clone()),
           snapshot_service: SnapshotServiceImpl::new(repository.clone(), blob_storage, event_publisher),
           query_service: FlowQueryServiceImpl::new(repository.clone()),
           metadata_service: MetadataServiceImpl::new(repository.clone()),
           repository }
  }
}

/// Configuración rápida con repositorio en memoria
///
/// Útil para testing y prototipado rápido.
pub fn create_in_memory_system() -> FlowSystem<crate::adapters::InMemoryFlowRepository> {
  let repository = Arc::new(crate::adapters::InMemoryFlowRepository::new());
  FlowSystem::new(repository)
}

#[cfg(test)]
mod tests {
  use crate::ports::{
    AddStepRequest, CreateBranchRequest, CreateSnapshotRequest, FlowManagementService, MetadataOperation, MetadataService,
    MetadataUpdate, SnapshotService,
  };

  use super::*;
  use serde_json::json;

  #[tokio::test]
  async fn test_system_facade() {
    let system = create_in_memory_system();

    // Crear flujo
    let flow_result = system.flow_service
                            .create_flow(Some("Test Flow".to_string()), Some("active".to_string()), json!({}), None)
                            .await
                            .unwrap();

    // Añadir paso
    let step_request = AddStepRequest { key: "test_step".to_string(),
                                        payload: json!({"content": "test"}),
                                        metadata: json!({}),
                                        command_id: None };

    let step_result = system.flow_service.add_step(&flow_result.flow_id, step_request).await.unwrap();

    assert_eq!(step_result.cursor, 1);

    // Obtener información del flujo
    let flow_info = system.flow_service.get_flow_info(&flow_result.flow_id).await.unwrap();

    assert_eq!(flow_info.step_count, 1);
    assert_eq!(flow_info.branch_count, 0);

    // Crear rama
    let branch_request = CreateBranchRequest { parent_flow_id: flow_result.flow_id,
                                               parent_cursor: 1,
                                               name: Some("Test Branch".to_string()),
                                               metadata: json!({}) };

    let branch_result = system.flow_service.create_branch(branch_request).await.unwrap();

    // Verificar rama
    let branch_info = system.flow_service.get_flow_info(&branch_result.branch_id).await.unwrap();

    assert_eq!(branch_info.metadata.parent_flow_id, Some(flow_result.flow_id));
  }

  #[tokio::test]
  async fn test_snapshot_operations() {
    let system = create_in_memory_system();

    // Crear flujo con pasos
    let flow_result = system.flow_service
                            .create_flow(Some("Snapshot Test".to_string()), Some("active".to_string()), json!({}), None)
                            .await
                            .unwrap();

    // Añadir varios pasos
    for i in 1..=5 {
      let step_request =
        AddStepRequest { key: format!("step_{}", i), payload: json!({"step": i}), metadata: json!({}), command_id: None };
      system.flow_service.add_step(&flow_result.flow_id, step_request).await.unwrap();
    }

    // Crear snapshot
    let snapshot_request = CreateSnapshotRequest { flow_id: flow_result.flow_id,
                                                   description: Some("Test checkpoint".to_string()),
                                                   metadata: json!({"test": true}) };

    let snapshot_result = system.snapshot_service.create_snapshot(snapshot_request).await.unwrap();

    assert_eq!(snapshot_result.cursor, 5);

    // Rehidratar desde snapshot
    let rehydration_result = system.snapshot_service.rehydrate_from_snapshot(&flow_result.flow_id).await.unwrap();

    assert_eq!(rehydration_result.snapshot_cursor, 5);
    assert_eq!(rehydration_result.final_cursor, 5);
  }

  #[tokio::test]
  async fn test_metadata_operations() {
    let system = create_in_memory_system();

    // Crear flujo
    let flow_result = system.flow_service
                            .create_flow(Some("Metadata Test".to_string()), Some("active".to_string()), json!({}), None)
                            .await
                            .unwrap();

    // Set metadata
    system.metadata_service.set_metadata(&flow_result.flow_id, "experiment_type", json!("synthesis")).await.unwrap();

    system.metadata_service.set_metadata(&flow_result.flow_id, "temperature", json!(60)).await.unwrap();

    // Get metadata
    let exp_type = system.metadata_service.get_metadata(&flow_result.flow_id, "experiment_type").await.unwrap();
    assert_eq!(exp_type, json!("synthesis"));

    // List keys
    let keys = system.metadata_service.list_metadata_keys(&flow_result.flow_id).await.unwrap();
    assert!(keys.contains(&"experiment_type".to_string()));
    assert!(keys.contains(&"temperature".to_string()));

    // Update batch
    let updates = vec![MetadataUpdate { key: "temperature".to_string(),
                                        operation: MetadataOperation::Increment(5),
                                        value: None },
                       MetadataUpdate { key: "status".to_string(),
                                        operation: MetadataOperation::Set(json!("completed")),
                                        value: None },];

    system.metadata_service.update_metadata_batch(&flow_result.flow_id, updates).await.unwrap();

    let temp = system.metadata_service.get_metadata(&flow_result.flow_id, "temperature").await.unwrap();
    assert_eq!(temp, json!(65));
  }
}
