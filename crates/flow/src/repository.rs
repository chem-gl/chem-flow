// Archivo: repository.rs
// Propósito: definir el trait `FlowRepository` y los traits auxiliares
// (`SnapshotStore`, `ArtifactStore`). Describe el contrato que deben
// implementar las persistencias (Postgres, in-memory, etc.).
use crate::domain::{FlowData, FlowMeta, PersistResult, SnapshotMeta, WorkItem};
use crate::errors::Result;
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Contrato mínimo del repositorio de flujos en el modelo basado en FlowData.
///
/// El repositorio persiste registros de datos del flujo (`FlowData`) en tiempo
/// real: cada registro contiene la información necesaria para reconstruir el
/// estado en un cursor dado y se guarda inmediatamente.
pub trait FlowRepository: Send + Sync {
    /// Obtiene metadatos ligeros del `flow`.
    fn get_flow_meta(&self, flow_id: &Uuid) -> Result<FlowMeta>;

    /// Crea un nuevo flow (insert en tabla `flows`). El repositorio genera
    /// el `flow_id` y completa los campos derivados (created_at, version,
    /// cursor). Se pasa sólo la información ergonomica: `name`, `status`
    /// y `metadata`.
    fn create_flow(&self, name: Option<String>, status: Option<String>, metadata: JsonValue) -> Result<Uuid>;

    /// Persiste un registro de datos para el flujo. `expected_version` permite
    /// controlar concurrencia (optimistic). Devuelve `PersistResult`.
    fn persist_data(&self, data: &FlowData, expected_version: i64) -> Result<PersistResult>;

    /// Lee registros de datos a partir de un cursor (exclusive), ordenados.
    fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> Result<Vec<FlowData>>;

    /// Devuelve metadata del último snapshot para este flow, si existe.
    fn load_latest_snapshot(&self, flow_id: &Uuid) -> Result<Option<SnapshotMeta>>;

    /// Carga snapshot por id: devuelve bytes serializados + metadata.
    fn load_snapshot(&self, snapshot_id: &Uuid) -> Result<(Vec<u8>, SnapshotMeta)>; // bytes, meta

    /// Guarda snapshot: escribe blob en object store y metadata en Postgres.
    fn save_snapshot(&self, flow_id: &Uuid, cursor: i64, state_ptr: &str, metadata: serde_json::Value) -> Result<Uuid>;

    /// Crea una rama (branch) a partir de `parent_flow_id` y `parent_cursor`.
    /// Firma ergonomica: el caller pasa `name`, `status`, `parent_cursor`
    /// y `metadata`. El repositorio genera el nuevo `flow_id`, copia los
    /// `FlowData` del padre hasta `parent_cursor` y persiste la nueva fila
    /// en `flows`. Devuelve el `Uuid` de la nueva rama.
    ///
    /// Debe hacerse de forma atómica por el repositorio concreto.
    fn create_branch(&self,
                     parent_flow_id: &Uuid,
                     name: Option<String>,
                     status: Option<String>,
                     parent_cursor: i64,
                     metadata: JsonValue)
                     -> Result<Uuid>;

    /// Verifica si existe una rama/flow con el id dado.
    fn branch_exists(&self, flow_id: &Uuid) -> Result<bool>;

    /// Cuenta cuántos pasos (`FlowData`) tiene un flow. Debe devolver
    /// -1 si el flow no existe, 0 si existe pero no tiene pasos.
    fn count_steps(&self, flow_id: &Uuid) -> Result<i64>;

    /// Elimina una rama y todas sus subramas (recursivo). Borra metadata,
    /// steps y snapshots asociados.
    fn delete_branch(&self, flow_id: &Uuid) -> Result<()>;

    /// Elimina todos los pasos y subramas a partir de un cursor dado
    /// (inclusive) en el flow `flow_id`.
    fn delete_from_step(&self, flow_id: &Uuid, from_cursor: i64) -> Result<()>;

    /// Lock ligero para actualizaciones (puede mapear a check de versión).
    fn lock_for_update(&self, flow_id: &Uuid, expected_version: i64) -> Result<bool>;

    /// Claim de trabajo para workers. Marca job como in-flight o devuelve
    /// `None`.
    fn claim_work(&self, worker_id: &str) -> Result<Option<WorkItem>>;

    /// Obtiene el estado (status) actual del flow.
    fn get_flow_status(&self, flow_id: &Uuid) -> Result<Option<String>>;

    /// Actualiza el estado (status) del flow. Devuelve el nuevo FlowMeta si se actualizó correctamente.
    fn set_flow_status(&self, flow_id: &Uuid, new_status: Option<String>) -> Result<FlowMeta>;
}

// Store traits para separar implementaciones de bajo nivel.
pub trait SnapshotStore: Send + Sync {
    /// Guarda bytes serializados y devuelve una key (p.ej. s3 key).
    fn save(&self, state: &[u8]) -> Result<String>;
    /// Carga bytes desde la key.
    fn load(&self, key: &str) -> Result<Vec<u8>>;
}

pub trait ArtifactStore: Send + Sync {
    /// Almacena blob y devuelve key.
    fn put(&self, blob: &[u8]) -> Result<String>;
    /// Recupera blob por key.
    fn get(&self, key: &str) -> Result<Vec<u8>>;
    /// Copia el blob si se necesita aislamiento (copy-on-write), devuelve nueva
    /// key.
    fn copy_if_needed(&self, src_key: &str) -> Result<String>;
}
