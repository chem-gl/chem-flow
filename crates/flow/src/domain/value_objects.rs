//! Value Objects del dominio - Objetos inmutables sin identidad
//!
//! Los value objects representan conceptos del dominio que se definen por sus
//! valores y no por su identidad. Son inmutables y pueden ser comparados por
//! valor.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

/// Identificador único de nodo en el árbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
  /// Genera un nuevo ID de nodo
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Crea desde UUID existente
  pub fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Obtiene el UUID interno
  pub fn as_uuid(&self) -> Uuid {
    self.0
  }
}

impl fmt::Display for NodeId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Node({})", self.0)
  }
}

impl Default for NodeId {
  fn default() -> Self {
    Self::new()
  }
}

/// Identificador único de rama
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(Uuid);

impl BranchId {
  /// Genera un nuevo ID de rama
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Crea desde UUID existente
  pub fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Obtiene el UUID interno
  pub fn as_uuid(&self) -> Uuid {
    self.0
  }
}

impl fmt::Display for BranchId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Branch({})", self.0)
  }
}

impl Default for BranchId {
  fn default() -> Self {
    Self::new()
  }
}

/// Datos de un paso en el flujo (evento inmutable)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowData {
  /// Identificador único del registro
  pub id: Uuid,
  /// Identificador del flujo al que pertenece
  pub flow_id: Uuid,
  /// Cursor secuencial monótono (0, 1, 2, ...)
  pub cursor: i64,
  /// Clave semántica del evento (ej: "step_state:step1")
  pub key: String,
  /// Payload del evento (datos arbitrarios en JSON)
  pub payload: Value,
  /// Metadata adicional (JSON arbitrario)
  pub metadata: Value,
  /// ID de comando para idempotencia (opcional)
  pub command_id: Option<Uuid>,
  /// Timestamp de creación
  pub created_at: DateTime<Utc>,
}

impl FlowData {
  /// Constructor para un nuevo registro de datos
  pub fn new(flow_id: Uuid,
             cursor: i64,
             key: impl Into<String>,
             payload: Value,
             metadata: Value,
             command_id: Option<Uuid>)
             -> Self {
    let key = key.into();
    assert!(cursor >= 0, "Cursor debe ser no negativo");
    assert!(!key.trim().is_empty(), "La clave no puede estar vacía");

    Self { id: Uuid::new_v4(), flow_id, cursor, key, payload, metadata, command_id, created_at: Utc::now() }
  }

  /// Obtiene el hash del contenido para verificación de duplicados
  pub fn get_content_hash(&self) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    // Hash only payload to detect duplicate content irrespective of the key
    // (tests expect payload-based duplication detection).
    self.payload.to_string().hash(&mut hasher);
    format!("{:x}", hasher.finish())
  }

  /// Verifica si este dato tiene el mismo contenido que otro
  pub fn has_same_content(&self, other: &FlowData) -> bool {
    self.key == other.key && self.payload == other.payload
  }

  /// Clona con nuevo flow_id (para ramificación)
  pub fn clone_for_branch(&self, new_flow_id: Uuid) -> Self {
    Self { id: Uuid::new_v4(),
           flow_id: new_flow_id,
           cursor: self.cursor,
           key: self.key.clone(),
           payload: self.payload.clone(),
           metadata: self.metadata.clone(),
           command_id: self.command_id,
           created_at: Utc::now() }
  }
}

/// Metadatos de un snapshot (apunta a un estado serializado)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMeta {
  /// Identificador único del snapshot
  pub id: Uuid,
  /// Identificador del flujo
  pub flow_id: Uuid,
  /// Cursor al que corresponde el snapshot
  pub cursor: i64,
  /// Puntero al estado almacenado (ej: clave S3)
  pub state_ptr: String,
  /// Metadata adicional
  pub metadata: Value,
  /// Timestamp de creación
  pub created_at: DateTime<Utc>,
}

impl SnapshotMeta {
  /// Constructor para metadata de snapshot
  pub fn new(flow_id: Uuid, cursor: i64, state_ptr: impl Into<String>, metadata: Value) -> Self {
    let state_ptr = state_ptr.into();
    assert!(cursor >= 0, "Cursor debe ser no negativo");
    assert!(!state_ptr.trim().is_empty(), "State pointer no puede estar vacío");

    Self { id: Uuid::new_v4(), flow_id, cursor, state_ptr, metadata, created_at: Utc::now() }
  }
}

/// Metadatos ligeros del flujo completo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowMetadata {
  /// Identificador único del flujo
  pub id: Uuid,
  /// Nombre del flujo (opcional)
  pub name: Option<String>,
  /// Estado actual (ej: "running", "completed")
  pub status: Option<String>,
  /// Creador (ej: usuario o sistema)
  pub created_by: Option<String>,
  /// Timestamp de creación del flujo
  pub created_at: DateTime<Utc>,
  /// Último cursor persistido
  pub current_cursor: i64,
  /// Versión actual para locking optimista
  pub current_version: i64,
  /// ID del flujo padre (para subflujos)
  pub parent_flow_id: Option<Uuid>,
  /// Cursor del padre donde se inició este flujo
  pub parent_cursor: Option<i64>,
  /// Metadata adicional
  pub metadata: Value,
}

impl FlowMetadata {
  /// Constructor para metadata de flujo
  pub fn new(name: Option<impl Into<String>>,
             status: Option<impl Into<String>>,
             created_by: Option<impl Into<String>>,
             parent_flow_id: Option<Uuid>,
             parent_cursor: Option<i64>,
             metadata: Value)
             -> Self {
    Self { id: Uuid::new_v4(),
           name: name.map(Into::into),
           status: status.map(Into::into),
           created_by: created_by.map(Into::into),
           created_at: Utc::now(),
           current_cursor: 0,
           current_version: 0,
           parent_flow_id,
           parent_cursor,
           metadata }
  }

  /// Incrementa la versión (para uso interno en persistencia)
  pub fn increment_version(&mut self) {
    self.current_version += 1;
  }

  /// Actualiza el cursor (solo si es mayor)
  pub fn update_cursor(&mut self, new_cursor: i64) {
    if new_cursor > self.current_cursor {
      self.current_cursor = new_cursor;
    }
  }
}

/// Metadatos específicos de una rama
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchMetadata {
  /// Nombre de la rama
  pub name: Option<String>,
  /// Estado de la rama
  pub status: Option<String>,
  /// Metadata adicional específica de la rama
  pub metadata: Value,
  /// Timestamp de creación
  pub created_at: DateTime<Utc>,
}

impl BranchMetadata {
  /// Constructor para metadata de rama
  pub fn new(name: Option<impl Into<String>>, status: Option<impl Into<String>>, metadata: Value) -> Self {
    Self { name: name.map(Into::into), status: status.map(Into::into), metadata, created_at: Utc::now() }
  }
}

/// Resultado de operaciones de persistencia con control de concurrencia
#[derive(Debug, Clone, PartialEq)]
pub enum PersistResult {
  /// Éxito con nueva versión asignada
  Ok { new_version: i64 },
  /// Conflicto de versión (otro proceso modificó concurrentemente)
  Conflict,
}

/// Elemento de trabajo para workers (para reclamar y procesar)
#[derive(Debug, Clone, PartialEq)]
pub struct WorkItem {
  /// ID del flujo a procesar
  pub flow_id: Uuid,
  /// Último cursor conocido
  pub last_cursor: i64,
  /// Puntero a snapshot (opcional, para rehidratación rápida)
  pub snapshot_ptr: Option<String>,
}

impl WorkItem {
  /// Constructor simple para WorkItem
  pub fn new(flow_id: Uuid, last_cursor: i64, snapshot_ptr: Option<impl Into<String>>) -> Self {
    Self { flow_id, last_cursor, snapshot_ptr: snapshot_ptr.map(Into::into) }
  }
}

/// Comando para añadir un paso a una rama
#[derive(Debug, Clone)]
pub struct AddStepCommand {
  /// ID de la rama donde añadir el paso
  pub branch_id: BranchId,
  /// Clave semántica del paso
  pub key: String,
  /// Contenido del paso
  pub payload: Value,
  /// Metadata adicional
  pub metadata: Value,
  /// ID de comando para idempotencia
  pub command_id: Option<Uuid>,
}

impl AddStepCommand {
  /// Constructor para comando de añadir paso
  pub fn new(branch_id: BranchId, key: impl Into<String>, payload: Value, metadata: Value) -> Self {
    Self { branch_id, key: key.into(), payload, metadata, command_id: Some(Uuid::new_v4()) }
  }

  /// Constructor sin command_id (no idempotente)
  pub fn new_non_idempotent(branch_id: BranchId, key: impl Into<String>, payload: Value, metadata: Value) -> Self {
    Self { branch_id, key: key.into(), payload, metadata, command_id: None }
  }
}

/// Comando para crear una nueva rama
#[derive(Debug, Clone)]
pub struct CreateBranchCommand {
  /// ID del flujo padre
  pub parent_flow_id: Uuid,
  /// Cursor en el padre donde ramificar
  pub parent_cursor: i64,
  /// Metadatos de la nueva rama
  pub metadata: Value,
  /// ID de comando para idempotencia
  pub command_id: Option<Uuid>,
}

impl CreateBranchCommand {
  /// Constructor para comando de crear rama
  pub fn new(parent_flow_id: Uuid, parent_cursor: i64, metadata: Value) -> Self {
    Self { parent_flow_id, parent_cursor, metadata, command_id: Some(Uuid::new_v4()) }
  }
}

/// Comando para eliminar una rama
#[derive(Debug, Clone)]
pub struct DeleteBranchCommand {
  /// ID de la rama a eliminar
  pub branch_id: BranchId,
  /// Si debe eliminar recursivamente las subramas
  pub recursive: bool,
  /// ID de comando para idempotencia
  pub command_id: Option<Uuid>,
}

impl DeleteBranchCommand {
  /// Constructor para comando de eliminar rama
  pub fn new(branch_id: BranchId, recursive: bool) -> Self {
    Self { branch_id, recursive, command_id: Some(Uuid::new_v4()) }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_node_id_creation_and_equality() {
    let id1 = NodeId::new();
    let id2 = NodeId::new();
    let id3 = NodeId::from_uuid(id1.as_uuid());

    assert_ne!(id1, id2);
    assert_eq!(id1, id3);
  }

  #[test]
  fn test_branch_id_creation_and_equality() {
    let id1 = BranchId::new();
    let id2 = BranchId::new();
    let id3 = BranchId::from_uuid(id1.as_uuid());

    assert_ne!(id1, id2);
    assert_eq!(id1, id3);
  }

  #[test]
  fn test_flow_data_creation() {
    let data = FlowData::new(Uuid::new_v4(),
                             1,
                             "test_key",
                             json!({"content": "test"}),
                             json!({"tags": ["test"]}),
                             Some(Uuid::new_v4()));

    assert_eq!(data.cursor, 1);
    assert_eq!(data.key, "test_key");
    assert!(data.command_id.is_some());
  }

  #[test]
  fn test_flow_data_content_hash() {
    let data1 = FlowData::new(Uuid::new_v4(), 1, "test_key", json!({"content": "test"}), json!({}), None);

    let data2 = FlowData::new(Uuid::new_v4(), 2, "test_key", json!({"content": "test"}), json!({}), None);

    // Mismo contenido debería generar mismo hash
    assert_eq!(data1.get_content_hash(), data2.get_content_hash());
    assert!(data1.has_same_content(&data2));
  }

  #[test]
  fn test_flow_data_clone_for_branch() {
    let original = FlowData::new(Uuid::new_v4(), 1, "test_key", json!({"content": "test"}), json!({}), None);

    let new_flow_id = Uuid::new_v4();
    let cloned = original.clone_for_branch(new_flow_id);

    assert_eq!(cloned.flow_id, new_flow_id);
    assert_eq!(cloned.cursor, original.cursor);
    assert_eq!(cloned.key, original.key);
    assert_eq!(cloned.payload, original.payload);
    assert_ne!(cloned.id, original.id); // Nuevo ID
  }

  #[test]
  fn test_flow_metadata_creation() {
    let metadata = FlowMetadata::new(Some("test_flow"), Some("active"), Some("test_user"), None, None, json!({}));

    assert_eq!(metadata.name, Some("test_flow".to_string()));
    assert_eq!(metadata.status, Some("active".to_string()));
    assert_eq!(metadata.current_cursor, 0);
    assert_eq!(metadata.current_version, 0);
  }

  #[test]
  fn test_flow_metadata_version_and_cursor_updates() {
    let mut metadata = FlowMetadata::new(Some("test_flow"), Some("active"), Some("test_user"), None, None, json!({}));

    metadata.increment_version();
    assert_eq!(metadata.current_version, 1);

    metadata.update_cursor(5);
    assert_eq!(metadata.current_cursor, 5);

    // No debería decrementar
    metadata.update_cursor(3);
    assert_eq!(metadata.current_cursor, 5);
  }

  #[test]
  fn test_commands_creation() {
    let branch_id = BranchId::new();
    let add_cmd = AddStepCommand::new(branch_id.clone(), "test_step", json!({"data": "test"}), json!({}));

    assert!(add_cmd.command_id.is_some());
    assert_eq!(add_cmd.key, "test_step");

    let create_cmd = CreateBranchCommand::new(Uuid::new_v4(), 5, json!({"purpose": "test"}));

    assert!(create_cmd.command_id.is_some());
    assert_eq!(create_cmd.parent_cursor, 5);

    let delete_cmd = DeleteBranchCommand::new(branch_id, true);
    assert!(delete_cmd.command_id.is_some());
    assert!(delete_cmd.recursive);
  }
}
