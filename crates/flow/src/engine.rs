// Archivo: engine.rs
// Propósito: implementar la estructura del `FlowEngine` (rehidratable).
//
// Nota: El motor aquí es un esqueleto responsable de rehidratación,
// decisión de snapshots y delegado de persistencia. La lógica concreta de
// aplicación de `FlowData` corre en una implementación externa y concreta.
use crate::domain::{FlowData, FlowMeta, PersistResult};
use crate::errors::{FlowError, Result};
use crate::repository::FlowRepository;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
/// Configuración simple del motor.
///
/// Actualmente vacío: sirve como placeholder para futuras opciones
/// (por ejemplo `snapshot_interval` o parámetros de rehidratación). Las
/// snapshots se gestionan hoy manualmente mediante `save_snapshot`.
pub struct FlowEngineConfig {
  // Por ahora no contiene campos; se deja para expansión futura.
}
/// Motor de ejecución rehidratable y resumible.
///
/// Responsabilidades principales:
/// - Rehidratarse a partir de snapshot + registros de datos
/// - Proveer utilidades para persistir/recuperar datos del flujo
/// - Decidir cuándo pedir snapshots
/// - Usar `FlowRepository` para persistir datos/snapshots
///
/// Nota sobre errores y concurrencia:
/// - Los métodos que realizan persistencia delegan en `FlowRepository` y
///   retornan `FlowError::Conflict` cuando la versión esperada no coincide.
/// - Este engine es deliberadamente pequeño: la ejecución de la lógica de pasos
///   (side-effects) debe implementarse fuera del crate para mantener separación
///   de responsabilidades. Motor de ejecución rehidratable que delega
///   persistencia en un `Arc<dyn FlowRepository>`.
///
/// La implementación es deliberadamente pequeña: se encarga de
/// mantener caches locales (snapshot/replay/idempotencia) y exponer
/// helpers para persistencia y branching. La lógica de pasos debe
/// residir fuera de este módulo/crate.
pub struct FlowEngine {
  /// Repositorio inyectado (trait object) para persistencia.
  pub repo: Arc<dyn FlowRepository>,
  /// Configuración del engine (placeholder para futuras opciones).
  #[allow(dead_code)]
  pub config: FlowEngineConfig,
  /// Cache simple para idempotencia (command_id vistos).
  pub idempotency_cache: Mutex<HashSet<Uuid>>,
  /// Último snapshot cargado (bytes) — sólo para rehidratación/inspección.
  pub last_snapshot: Mutex<Option<Vec<u8>>>,
  /// Últimos FlowData recibidos durante rehidratación (replay).
  pub last_replay: Mutex<Vec<FlowData>>,
}
impl FlowEngine {
  /// Crea una nueva instancia del motor.
  ///
  /// Parámetros:
  /// - `repo`: repositorio inyectado como `Arc<dyn FlowRepository>`.
  /// - `config`: configuración del engine (actualmente sin campos).
  pub fn new(repo: Arc<dyn FlowRepository>, _config: FlowEngineConfig) -> Self {
    Self { repo,
           config: FlowEngineConfig {},
           idempotency_cache: Mutex::new(HashSet::new()),
           last_snapshot: Mutex::new(None),
           last_replay: Mutex::new(Vec::new()) }
  }
  /// Rehidrata el motor: aplica opcional `snapshot_state` y luego reconstruye
  /// el estado a partir de los registros de datos persistidos. La función
  /// recibe bytes del snapshot (si existe) y una lista de `FlowData` para
  /// reconstruir el estado local.
  /// Rehidrata el engine guardando localmente el snapshot (si existe)
  /// y la lista de `FlowData` que constituyen el replay.
  ///
  /// Nota: esta función no aplica lógica de negocio ni ejecuta pasos;
  /// sólo almacena los datos para inspección o para que el wrapper
  /// (implementación concreta) pueda reconstituir su estado.
  pub fn rehydrate(&self, snapshot_state: Option<&[u8]>, data: &[FlowData]) -> Result<()> {
    // Guardar snapshot (si hay)
    let mut snap_lock = self.last_snapshot.lock().map_err(|e| FlowError::Storage(format!("mutex poisoned: {:?}", e)))?;
    *snap_lock = snapshot_state.map(|b| b.to_vec());
    // Guardar replay
    let mut replay = self.last_replay.lock().map_err(|e| FlowError::Storage(format!("mutex poisoned: {:?}", e)))?;
    replay.clear();
    replay.extend_from_slice(data);
    Ok(())
  }
  // --- Eliminadas funciones no-operativas: resume, execute_next y
  // request_snapshot_if_needed. Este `FlowEngine` expone únicamente
  // helpers de persistencia/lectura/snapshots/branching. La ejecución
  // concreta de pasos debe residir fuera de este crate.
  /// Helper ergonómico: crea y persiste un `FlowData` a partir de
  /// parámetros. Calcula el próximo `cursor` a partir de `FlowMeta` y
  /// delega en `persist_data`.
  ///
  /// Input:
  /// - `flow_id`: identificador del flow.
  /// - `key`, `payload`, `metadata`: contenido del `FlowData`.
  /// - `command_id`: opcional, para idempotencia.
  /// - `expected_version`: versión esperada para locking optimista.
  ///
  /// Output:
  /// - `Ok(PersistResult::Ok)` con la nueva versión cuando se persiste con
  ///   éxito.
  /// - `Ok(PersistResult::Conflict)` si la versión no coincide.
  ///
  /// Retorna `PersistResult` devuelto por el repositorio.
  pub fn append(&self,
                flow_id: Uuid,
                key: &str,
                payload: serde_json::Value,
                metadata: serde_json::Value,
                command_id: Option<Uuid>,
                expected_version: i64)
                -> Result<PersistResult> {
    let meta: FlowMeta = self.repo.get_flow_meta(&flow_id)?;
    let next_cursor = meta.current_cursor + 1;
    let data = FlowData { id: Uuid::new_v4(),
                          flow_id,
                          cursor: next_cursor,
                          key: key.to_string(),
                          payload,
                          metadata,
                          command_id,
                          created_at: Utc::now() };
    self.persist_data(&data, expected_version)
  }
  /// Helper ergonómico: crea un nuevo flow delegando al repositorio.
  /// Retorna el `Uuid` generado.
  /// Crea un nuevo flow delegando en el repositorio.
  pub fn start_flow(&self, name: Option<String>, status: Option<String>, metadata: serde_json::Value) -> Result<Uuid> {
    self.repo.create_flow(name, status, metadata)
  }
  /// Claim/obtener trabajo para un worker identificado por `worker_id`.
  /// Retorna `Some(WorkItem)` si hay trabajo, `None` si no.
  /// Reclama un `WorkItem` para el worker `worker_id` si hay trabajo.
  pub fn claim_work(&self, worker_id: &str) -> Result<Option<crate::domain::WorkItem>> {
    self.repo.claim_work(worker_id)
  }
  /// Verifica si existe una rama con el id dado.
  /// Comprueba si existe una rama con el id indicado.
  pub fn branch_exists(&self, flow_id: &Uuid) -> Result<bool> {
    self.repo.branch_exists(flow_id)
  }
  /// Cuenta cuántos pasos tiene un flow. -1 si no existe.
  /// Devuelve el número de pasos de un flow (o -1 si no existe).
  pub fn count_steps(&self, flow_id: &Uuid) -> Result<i64> {
    self.repo.count_steps(flow_id)
  }
  /// Elimina una rama y todas sus subramas.
  /// Elimina una rama y todas sus subramas.
  pub fn delete_branch(&self, flow_id: &Uuid) -> Result<()> {
    self.repo.delete_branch(flow_id)
  }
  /// Elimina todos los pasos y subramas a partir de un cursor dado.
  /// Elimina todos los pasos y subramas a partir de un cursor dado.
  pub fn delete_from_step(&self, flow_id: &Uuid, from_cursor: i64) -> Result<()> {
    self.repo.delete_from_step(flow_id, from_cursor)
  }
  /// Lectura directa de `FlowData` desde el repositorio a partir de un
  /// `from_cursor` (exclusive).
  /// Lee `FlowData` a partir de `from_cursor` (exclusive).
  pub fn get_items(&self, flow_id: &Uuid, from_cursor: i64) -> Result<Vec<FlowData>> {
    self.repo.read_data(flow_id, from_cursor)
  }
  /// Delegado para guardar snapshot: escribe metadata + state_ptr.
  /// Guarda un snapshot delegando en el repositorio (metadata + state_ptr).
  pub fn save_snapshot(&self, flow_id: &Uuid, cursor: i64, state_ptr: &str, metadata: serde_json::Value) -> Result<Uuid> {
    self.repo.save_snapshot(flow_id, cursor, state_ptr, metadata)
  }
  /// Crear una rama (branch) a partir de `parent_flow_id` y `parent_cursor`.
  /// Firma ergonomica: se pasa `name`, `status`, `parent_cursor` y
  /// `metadata`. El repositorio generará el nuevo id y copiará los datos
  /// necesarios.
  /// Crea una nueva rama a partir de `parent_flow_id` y `parent_cursor`.
  pub fn new_branch(&self, parent_flow_id: &Uuid, parent_cursor: i64, metadata: serde_json::Value) -> Result<Uuid> {
    self.repo.create_branch(parent_flow_id, parent_cursor, metadata)
  }
  /// Persiste un FlowData usando el repositorio y el control optimista de
  /// versiones. Simple delegación al `FlowRepository`.
  /// Persiste un `FlowData` usando control optimista de versiones.
  pub fn persist_data(&self, data: &FlowData, expected_version: i64) -> Result<PersistResult> {
    self.repo.persist_data(data, expected_version)
  }
}
