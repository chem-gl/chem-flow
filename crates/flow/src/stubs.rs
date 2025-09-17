// Archivo: stubs.rs
// Propósito: implementaciones en memoria para pruebas y wiring rápido.
//
// Incluye un repositorio en memoria (`InMemoryFlowRepository`), un pool de
// workers y stubs para stores. Estas implementaciones no son durables y se
// usan para demos o pruebas locales.
use crate::domain::{FlowData, FlowMeta, PersistResult, SnapshotMeta, WorkItem};
use crate::errors::{FlowError, Result};
use crate::repository::{ArtifactStore, FlowRepository, SnapshotStore};
use chrono::Utc;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

/// Pool simple en memoria para encolar y reclamar `WorkItem`.
///
/// Uso pensado para pruebas locales y ejemplos. No garantiza durabilidad
/// ni comportamiento distribuido.
#[derive(Debug)]
pub struct InMemoryWorkerPool {
    queue: Mutex<VecDeque<WorkItem>>,
}

impl InMemoryWorkerPool {
    /// Crea un nuevo pool de workers en memoria.
    pub fn new() -> Self {
        Self { queue: Mutex::new(VecDeque::new()) }
    }

    /// Encola un item de trabajo para ser reclamado por un worker.
    pub fn enqueue(&self, item: WorkItem) {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).push_back(item);
    }

    /// Reclama el siguiente item de trabajo disponible, si existe.
    pub fn claim(&self) -> Option<WorkItem> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front()
    }
}

pub struct GateService {
    /// Mapa (flow_id, step_id) -> open?
    gates: Mutex<HashMap<(Uuid, String), bool>>,
}

impl GateService {
    /// Crea un nuevo servicio de gates en memoria.
    pub fn new() -> Self {
        Self { gates: Mutex::new(HashMap::new()) }
    }

    /// Abre una gate para un step específico en un flow.
    pub fn open_gate(&self, flow_id: Uuid, step_id: &str, _reason: &str) {
        self.gates
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((flow_id, step_id.to_string()), true);
    }

    /// Cierra una gate para un step específico en un flow.
    pub fn close_gate(&self, flow_id: Uuid, step_id: &str, _input: serde_json::Value) {
        self.gates
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((flow_id, step_id.to_string()), false);
    }

    /// Verifica si una gate está abierta para un step específico en un flow.
    pub fn is_open(&self, flow_id: Uuid, step_id: &str) -> bool {
        *self.gates
             .lock()
             .unwrap_or_else(|e| e.into_inner())
             .get(&(flow_id, step_id.to_string()))
             .unwrap_or(&false)
    }
}

// Minimal in-memory repository for wiring examples (not durable)
pub struct InMemoryFlowRepository {
    /// Metadatos de flows indexados por `flow_id`.
    flows: Mutex<HashMap<Uuid, FlowMeta>>,
    /// Registros de FlowData por flow.
    steps: Mutex<HashMap<Uuid, Vec<FlowData>>>,
    /// Snapshots metadata por snapshot id.
    snapshots: Mutex<HashMap<Uuid, SnapshotMeta>>,
}

impl InMemoryFlowRepository {
    /// Crea una nueva instancia del repositorio en memoria.
    pub fn new() -> Self {
        Self { flows: Mutex::new(HashMap::new()),
               steps: Mutex::new(HashMap::new()),
               snapshots: Mutex::new(HashMap::new()) }
    }

    /// Helper para mapear `Mutex::lock()` en un `Result` con
    /// `FlowError::Storage`.
    fn lock<'a, T>(&'a self, m: &'a Mutex<T>) -> std::result::Result<MutexGuard<'a, T>, FlowError> {
        m.lock().map_err(|e| FlowError::Storage(format!("mutex poisoned: {:?}", e)))
    }
}

impl Default for InMemoryFlowRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowRepository for InMemoryFlowRepository {
    /// Obtiene los metadatos ligeros de un flow en memoria.
    /// Retorna `NotFound` si el flow no existe.
    fn get_flow_meta(&self, flow_id: &Uuid) -> Result<FlowMeta> {
        let flows = self.lock(&self.flows)?;
        flows.get(flow_id)
             .cloned()
             .ok_or(FlowError::NotFound(format!("flow {}", flow_id)))
    }

    /// Crea un nuevo flow en memoria. Inserta la metadata y devuelve el id.
    fn create_flow(&self, name: Option<String>, status: Option<String>, metadata: serde_json::Value) -> Result<Uuid> {
        // Generar id y metadatos básicos
        let id = Uuid::new_v4();
        let meta = FlowMeta { id,
                              name,
                              status,
                              created_by: None,
                              created_at: Utc::now(),
                              current_cursor: 0,
                              current_version: 0,
                              parent_flow_id: None,
                              parent_cursor: None,
                              metadata };
        self.lock(&self.flows)?.insert(id, meta.clone());
        Ok(id)
    }

    /// Devuelve el último `SnapshotMeta` para el flow si existe.
    fn load_latest_snapshot(&self, flow_id: &Uuid) -> Result<Option<SnapshotMeta>> {
        // Elegimos el snapshot de mayor cursor para el flow (si existe).
        let snaps = self.lock(&self.snapshots)?;
        Ok(snaps.values()
                .filter(|s| &s.flow_id == flow_id)
                .max_by_key(|s| s.cursor)
                .cloned())
    }

    /// Carga un snapshot por id. Retorna los bytes (simulados) y la metadata.
    fn load_snapshot(&self, snapshot_id: &Uuid) -> Result<(Vec<u8>, SnapshotMeta)> {
        let snaps = self.lock(&self.snapshots)?;
        let meta = snaps.get(snapshot_id)
                        .cloned()
                        .ok_or(FlowError::NotFound("snapshot".into()))?;
        Ok((vec![], meta))
    }

    /// Lee los `FlowData` para un `flow_id` a partir de `from_cursor`
    /// (exclusive), ordenados por cursor.
    fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> Result<Vec<FlowData>> {
        let steps = self.lock(&self.steps)?;
        Ok(steps.get(flow_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|d| d.cursor > from_cursor)
                .collect())
    }

    /// Persiste un `FlowData` aplicando control optimista por
    /// `expected_version` y deduplicación por `command_id` cuando está
    /// presente.
    fn persist_data(&self, data: &FlowData, expected_version: i64) -> Result<PersistResult> {
        let mut flows = self.lock(&self.flows)?;
        let mut steps = self.lock(&self.steps)?;
        let flow_meta = flows.get_mut(&data.flow_id).ok_or(FlowError::NotFound("flow".into()))?;
        // Optimistic concurrency: check expected_version
        if flow_meta.current_version != expected_version {
            return Ok(PersistResult::Conflict);
        }

        // Idempotency: if command_id present, ensure we don't duplicate
        if let Some(cmd_id) = data.command_id {
            if let Some(existing) = steps.get(&data.flow_id) {
                if existing.iter().any(|d| d.command_id == Some(cmd_id)) {
                    // Return current version (no change)
                    return Ok(PersistResult::Ok { new_version: flow_meta.current_version });
                }
            }
        }

        // Basic validations: ensure cursor monotonicity
        if data.cursor <= flow_meta.current_cursor {
            return Err(FlowError::Conflict(format!("cursor {} not greater than current {}",
                                                   data.cursor, flow_meta.current_cursor)));
        }

        // Persist the data
        let list = steps.entry(data.flow_id).or_default();
        list.push(data.clone());
        flow_meta.current_version = flow_meta.current_version.saturating_add(1);
        flow_meta.current_cursor = data.cursor;

        Ok(PersistResult::Ok { new_version: flow_meta.current_version })
    }

    /// Guarda metadata de snapshot en memoria. El `state_ptr` es solamente
    /// un string/clave simbólica en esta implementación.
    fn save_snapshot(&self, flow_id: &Uuid, seq: i64, state_ptr: &str, metadata: serde_json::Value) -> Result<uuid::Uuid> {
        let id = Uuid::new_v4();
        let meta = SnapshotMeta { id,
                                  flow_id: *flow_id,
                                  cursor: seq,
                                  state_ptr: state_ptr.to_string(),
                                  metadata,
                                  created_at: Utc::now() };
        self.lock(&self.snapshots)?.insert(id, meta);
        Ok(id)
    }

    /// Crea una nueva rama en memoria: genera `new_id`, copia todos los
    /// `FlowData` del padre con `cursor <= parent_cursor` y añade un
    /// `BranchCreated` al final. Devuelve `new_id`.
    fn create_branch(&self,
                     parent_flow_id: &Uuid,
                     name: Option<String>,
                     status: Option<String>,
                     parent_cursor: i64,
                     metadata: serde_json::Value)
                     -> Result<Uuid> {
        let new_id = Uuid::new_v4();

        // clonar metadata del padre si existe
        let parent_meta = {
            let flows = self.lock(&self.flows)?;
            flows.get(parent_flow_id).cloned()
        };

        let meta = if let Some(mut pm) = parent_meta {
            pm.id = new_id;
            pm.parent_flow_id = Some(*parent_flow_id);
            pm.parent_cursor = Some(parent_cursor);
            pm.current_cursor = parent_cursor;
            pm.current_version = 0;
            // override ergonomics if provided
            if name.is_some() {
                pm.name = name.clone();
            }
            if status.is_some() {
                pm.status = status.clone();
            }
            if metadata != serde_json::json!({}) {
                pm.metadata = metadata.clone();
            }
            pm
        } else {
            FlowMeta { id: new_id,
                       name: name.or_else(|| Some(format!("branch-of-{}", parent_flow_id))),
                       status: status.or(Some("queued".into())),
                       created_by: None,
                       created_at: Utc::now(),
                       current_cursor: parent_cursor,
                       current_version: 0,
                       parent_flow_id: Some(*parent_flow_id),
                       parent_cursor: Some(parent_cursor),
                       metadata }
        };

        // insert new flow meta
        self.lock(&self.flows)?.insert(new_id, meta.clone());

        // copy steps from parent until parent_cursor
        let mut steps = self.lock(&self.steps)?;
        if let Some(parent_steps) = steps.get(parent_flow_id).cloned() {
            let copied: Vec<FlowData> = parent_steps.into_iter()
                                                    .filter(|d| d.cursor <= parent_cursor)
                                                    .map(|mut d| {
                                                        d.id = Uuid::new_v4();
                                                        d.flow_id = new_id;
                                                        d
                                                    })
                                                    .collect();
            let entry = steps.entry(new_id).or_default();
            entry.extend(copied);
        } else {
            // No hay pasos previos; esto es válido (branch vacío)
            println!("[stub] no parent steps found for {}", parent_flow_id);
        }

        // Create a BranchCreated step as the next cursor
        let st = FlowData { id: Uuid::new_v4(),
                            flow_id: new_id,
                            cursor: parent_cursor + 1,
                            key: "BranchCreated".into(),
                            payload: serde_json::json!({"parent": parent_flow_id}),
                            metadata: serde_json::json!({}),
                            command_id: None,
                            created_at: Utc::now() };
        // Append the BranchCreated record for the new branch.
        steps.entry(new_id).or_default().push(st);

        Ok(new_id)
    }

    /// Lock ligero: en memoria siempre devuelve true (no concurrencia real).
    fn lock_for_update(&self, _flow_id: &Uuid, _expected_version: i64) -> Result<bool> {
        // In-memory lock: check that the flow exists and version matches expected.
        let flows = self.lock(&self.flows)?;
        if let Some(meta) = flows.get(_flow_id) {
            Ok(meta.current_version == _expected_version)
        } else {
            Err(FlowError::NotFound(format!("flow {}", _flow_id)))
        }
    }

    /// Claim de trabajo: en memoria no hay implementación de cola
    /// persistente, por lo que siempre devuelve `None`.
    fn claim_work(&self, _worker_id: &str) -> Result<Option<WorkItem>> {
        Ok(None)
    }

    /// Verifica si existe una rama/flow con el id dado.
    fn branch_exists(&self, flow_id: &Uuid) -> Result<bool> {
        let flows = self.lock(&self.flows)?;
        Ok(flows.contains_key(flow_id))
    }

    /// Cuenta cuántos pasos tiene un flow. -1 si no existe.
    fn count_steps(&self, flow_id: &Uuid) -> Result<i64> {
        let flows = self.lock(&self.flows)?;
        if !flows.contains_key(flow_id) {
            return Ok(-1);
        }
        let steps = self.lock(&self.steps)?;
        let cnt = steps.get(flow_id).map(|v| v.len() as i64).unwrap_or(0);
        Ok(cnt)
    }

    /// Elimina una rama y todas sus subramas (recursivo). Borra metadata,
    /// steps y snapshots asociados.
    fn delete_branch(&self, flow_id: &Uuid) -> Result<()> {
        // collect children recursively
        let mut to_delete: Vec<Uuid> = Vec::new();
        {
            let flows = self.lock(&self.flows)?;
            if !flows.contains_key(flow_id) {
                return Err(FlowError::NotFound(format!("flow {}", flow_id)));
            }
        }
        // BFS
        to_delete.push(*flow_id);
        let mut idx = 0;
        while idx < to_delete.len() {
            let current = to_delete[idx];
            idx += 1;
            let flows = self.lock(&self.flows)?;
            for (id, meta) in flows.iter() {
                if let Some(parent) = meta.parent_flow_id {
                    if parent == current {
                        to_delete.push(*id);
                    }
                }
            }
        }

        // perform deletions
        let mut flows = self.lock(&self.flows)?;
        let mut steps = self.lock(&self.steps)?;
        let mut snaps = self.lock(&self.snapshots)?;
        for id in to_delete.iter() {
            flows.remove(id);
            steps.remove(id);
            // remove snapshots for this flow
            let keys: Vec<Uuid> = snaps.iter().filter(|(_, s)| s.flow_id == *id).map(|(k, _)| *k).collect();
            for k in keys {
                snaps.remove(&k);
            }
        }

        Ok(())
    }

    /// Elimina todos los pasos y subramas a partir de un cursor dado
    /// (inclusive) en el flow `flow_id`.
    fn delete_from_step(&self, flow_id: &Uuid, from_cursor: i64) -> Result<()> {
        // check exists
        let flows = self.lock(&self.flows)?;
        let _meta = flows.get(flow_id)
                         .cloned()
                         .ok_or(FlowError::NotFound(format!("flow {}", flow_id)))?;
        drop(flows);

        // delete steps with cursor >= from_cursor
        let mut steps = self.lock(&self.steps)?;
        if let Some(vec) = steps.get_mut(flow_id) {
            vec.retain(|d| d.cursor < from_cursor);
        }
        drop(steps);

        // delete subbranches whose parent_cursor >= from_cursor recursively
        let mut to_delete: Vec<Uuid> = Vec::new();
        let flows = self.lock(&self.flows)?;
        for (id, fm) in flows.iter() {
            if let Some(p) = fm.parent_flow_id {
                if p == *flow_id {
                    if let Some(pc) = fm.parent_cursor {
                        if pc >= from_cursor {
                            to_delete.push(*id);
                        }
                    }
                }
            }
        }
        drop(flows);

        for id in to_delete.iter() {
            // reuse delete_branch to remove subtrees
            self.delete_branch(id)?;
        }

        Ok(())
    }
}
impl SnapshotStore for InMemoryFlowRepository {
    /// Guarda bytes serializados y devuelve una key representativa.
    fn save(&self, _state: &[u8]) -> Result<String> {
        Ok("inmem".into())
    }
    /// Carga bytes desde la key (no persistido en este stub).
    fn load(&self, _key: &str) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}

impl ArtifactStore for InMemoryFlowRepository {
    /// Almacena un blob y devuelve una key simbólica.
    fn put(&self, _blob: &[u8]) -> Result<String> {
        Ok("inmem-artifact".into())
    }
    /// Recupera blob por key (no persistido en este stub).
    fn get(&self, _key: &str) -> Result<Vec<u8>> {
        Ok(vec![])
    }
    /// Copia el blob si se necesita aislamiento. En este stub retorna la misma
    /// key.
    fn copy_if_needed(&self, src_key: &str) -> Result<String> {
        Ok(src_key.to_string())
    }
}
