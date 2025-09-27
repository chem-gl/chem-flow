//engine.rs
// Archivo: domain.rs
// Propósito: definir los tipos de dominio principales del crate `flow`.
//
// Aquí se definen `FlowData`, `FlowMeta`, `SnapshotMeta`, resultados de
// persistencia y estructuras auxiliares. Comentarios y nombres en español.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
/// Registro de datos del flujo (`FlowData`).
///
/// `FlowData` representa un evento o registro persistente asociado a un
/// flujo. Cada registro contiene un `cursor` monótono que permite ordenar
/// los eventos; además lleva un `payload` JSON y `metadata` libre.
///
/// Importante: este crate no ejecuta la lógica de negocio del step — solo
/// proporciona la estructura para persistir y recuperar los registros.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowData {
  /// Identificador único del registro de datos.
  pub id: Uuid,
  /// Identificador del flow al que pertenece.
  pub flow_id: Uuid,
  /// Cursor o secuencia lógica dentro del flujo (monótono).
  pub cursor: i64,
  /// Llave o tipo semántico del dato (ej. "step-result", "input",
  /// "artifact-ref").
  pub key: String,
  /// Payload con los datos a persistir para este cursor.
  pub payload: serde_json::Value,
  /// Metadata opcional (ej. quién lo creó, tags).
  pub metadata: serde_json::Value,
  /// Identificador de comando para idempotencia (opcional).
  pub command_id: Option<Uuid>,
  /// Marca temporal de creación.
  pub created_at: DateTime<Utc>,
}
/// Metadata de snapshot: metadata en Postgres y `state_ptr` apunta a blob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
  pub id: Uuid,
  pub flow_id: Uuid,
  pub cursor: i64,
  pub state_ptr: String,
  pub metadata: serde_json::Value,
  pub created_at: DateTime<Utc>,
}
/// Metadatos ligeros del agregado `flow` guardados en Postgres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMeta {
  pub id: Uuid,
  pub name: Option<String>,
  pub status: Option<String>,
  pub created_by: Option<String>,
  pub created_at: DateTime<Utc>,
  /// Cursor lógico (último step persistido)
  pub current_cursor: i64,
  /// Versión para locking optimista (incremental por persistencia)
  pub current_version: i64,
  pub parent_flow_id: Option<Uuid>,
  pub parent_cursor: Option<i64>,
  pub metadata: serde_json::Value,
}
/// Resultado de operaciones de persistencia que requieren control de versiones.
#[derive(Debug, Clone)]
pub enum PersistResult {
  /// OK con nueva versión.
  Ok {
    new_version: i64,
  },
  Conflict,
}
/// Item de trabajo que un worker puede reclamar. Contiene referencias para
/// rehidratación (último cursor y pointer a snapshot si existe).
#[derive(Debug, Clone)]
pub struct WorkItem {
  /// Identificador del flow a procesar.
  pub flow_id: Uuid,
  /// Último cursor conocido para el worker.
  pub last_cursor: i64,
  /// Snapshot pointer si existe.
  pub snapshot_ptr: Option<String>,
}
