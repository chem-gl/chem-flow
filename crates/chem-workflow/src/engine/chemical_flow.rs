use crate::{workflow_type::WorkflowType, WorkflowError};
use chem_domain::DomainRepository;
use flow::repository::FlowRepository;
use serde_json::Value as JsonValue;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

/// Trait genérico para motores de flujo químicos.
///
/// Este trait define la interfaz mínima que debe exponer un motor de
/// workflow químico. El motor es responsable de ejecutar pasos,
/// manejar snapshots y crear ramas. No asume la implementación concreta
/// del dominio (moléculas/familias), por eso opera con `serde_json::Value`
/// para la entrada y salida y expone métodos auxiliares para persistencia
/// y metadatos.
pub trait ChemicalFlowEngine: Send + Sync {
  /// Identificador del flow asociado a esta instancia.
  fn id(&self) -> Uuid;

  /// Aplica un snapshot serializado para rehidratar el estado del motor.
  ///
  /// El snapshot debe ser un `JsonValue` producido por `snapshot()` y
  /// representar el estado completo necesario para reanudar la ejecucion.
  fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>>;

  /// Extrae el estado serializado listo para almacenarse como snapshot.
  ///
  /// Debe producir un `JsonValue` autocontenido que `apply_snapshot`
  /// pueda volver a aplicar.
  fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>>;

  /// Devuelve el `WorkflowType` concreto implementado por este engine.
  ///
  /// Las implementaciones deben retornar la variante del enum que
  /// corresponde con el motor concreto (por ejemplo `CadmaFlow` ->
  /// `WorkflowType::Cadma`). Esto permite registro y filtrado por tipo.
  fn engine_workflow_type() -> WorkflowType
    where Self: Sized;

  /// Construye una instancia concreta del engine proporcionando los
  /// repositorios necesarios. Usado por la fábrica para instanciar tipos
  /// concretos genéricamente.
  fn construct_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized;

  /// Construye y rehidrata una instancia concreta usando los repositorios
  /// y el `flow id` provistos. Las implementaciones deben aplicar cualquier
  /// lógica de rehidratación específica del flujo (snapshots, replay de
  /// eventos) aquí y devolver un engine listo para su uso.
  fn rehydrate_with_repos(id: Uuid,
                          flow_repo: Arc<dyn FlowRepository>,
                          domain_repo: Arc<dyn DomainRepository>)
                          -> Result<Self, WorkflowError>
    where Self: Sized;

  /// Crea una rama (branch) a partir de un cursor/version dado.
  ///
  /// - `parent_cursor`: cursor/punto de corte desde el cual se bifurca.
  /// - `name`, `status`: metadatos opcionales para la nueva rama.
  fn create_branch(&self, parent_cursor: i64, name: Option<String>, status: Option<String>) -> Result<Uuid, Box<dyn Error>>;

  /// Devuelve una referencia al repositorio de flujo asociado al engine.
  fn flow_repo(&self) -> &Arc<dyn FlowRepository>;

  /// Obtiene una copia del `Arc` del repositorio de flujo.
  /// Útil cuando se necesita pasar el repositorio a helpers/steps.
  fn get_flow_repo(&self) -> Arc<dyn FlowRepository> {
    self.flow_repo().clone()
  }

  /// Obtiene un metadato (`FlowMeta`) por clave para este flow.
  /// Retorna `JsonValue::Null` si no existe.
  fn get_metadata(&self, key: &str) -> Result<JsonValue, WorkflowError> {
    self.flow_repo().get_meta(&self.id(), key).map_err(|e| WorkflowError::Persistence(format!("get_meta error: {}", e)))
  }

  /// Establece un metadato para el flow asociado.
  fn set_metadata(&self, key: &str, value: JsonValue) -> Result<(), WorkflowError> {
    self.flow_repo()
        .set_meta(&self.id(), key, value)
        .map_err(|e| WorkflowError::Persistence(format!("set_meta error: {}", e)))
  }

  /// Elimina un metadato identificado por `key` para este flow.
  fn del_metadata(&self, key: &str) -> Result<(), WorkflowError> {
    self.flow_repo().del_meta(&self.id(), key).map_err(|e| WorkflowError::Persistence(format!("del_meta error: {}", e)))
  }
  fn count_steps_initialized(&self) -> Result<i64, WorkflowError> {
    self.flow_repo().count_steps(&self.id()).map_err(|e| WorkflowError::Persistence(format!("count_steps error: {}", e)))
  }
}
