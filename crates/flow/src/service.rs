// Archivo: service.rs
// Propósito: implementar `FlowService`, una capa orquestadora que expone
// operaciones de alto nivel sobre flujos (crear rama, iniciar flow,
// reclamar trabajo, persistir/leer FlowData). Esta capa debe ser invocada
// desde handlers HTTP o desde workers.
use crate::domain::{FlowData, WorkItem};
use crate::engine::{FlowEngine, FlowEngineConfig};
use crate::errors::Result;
use crate::repository::FlowRepository;
// chrono/serde_json no se usan directamente aquí
use std::sync::Arc;
use uuid::Uuid;

/// Servicio de alto nivel que expone la API de operaciones sobre flujos.
///
/// Esta capa orquesta el repositorio y el motor. Está pensada para ser
/// invocada desde un handler HTTP o desde workers.
pub struct FlowService<R> where R: FlowRepository
{
    repo: Arc<R>,
    engine: Arc<FlowEngine<R>>,
}

impl<R> FlowService<R> where R: FlowRepository + 'static
{
    /// Crea el servicio inyectando el `FlowRepository` y la configuración del
    /// motor. El `FlowEngine` se construye internamente y se reusa.
    pub fn new(repo: Arc<R>, engine_config: FlowEngineConfig) -> Self {
        let engine = Arc::new(FlowEngine::new(repo.clone(), engine_config));
        Self { repo, engine }
    }

    /// Inicia un nuevo flow: crea la fila en `flows` de forma ergonómica.
    /// El repositorio generará el `flow_id`. Se pasan sólo `name`,
    /// `status` y `metadata`.
    pub fn start_flow(&self, name: Option<String>, status: Option<String>, metadata: serde_json::Value) -> Result<Uuid> {
        self.repo.create_flow(name, status, metadata)
    }

    /// Crea una rama a partir de un snapshot o una secuencia. El parámetro
    /// `_snapshot_id_or_seq` puede ser id de snapshot o un seq; la función
    /// debe cargar el estado y crear la nueva fila en la BD de forma atómica.
    pub fn create_branch(&self,
                                       parent_flow_id: Uuid,
                                       name: Option<String>,
                                       status: Option<String>,
                                       parent_cursor: i64,
                                       metadata: serde_json::Value)
                                       -> Result<Uuid> {
        // Delegar la creación de la rama al repositorio; el repositorio
        // generará el id y copiará los datos necesarios.
        self.repo
            .create_branch(&parent_flow_id, name, status, parent_cursor, metadata)
    }
    /// En este modelo no existe interacción humana explícita ni eventos.
    /// El crate persiste registros de datos (`FlowData`) en tiempo real. Para
    /// crear o persistir un dato asociado al flujo, usar `persist_data` del
    /// `FlowEngine`. (Este método se mantiene ausente intencionalmente.)

    /// Claim/obtener trabajo para un worker identificado por `worker_id`.
    /// Retorna `Some(WorkItem)` si hay trabajo, `None` si no.
    pub fn claim_work(&self, worker_id: &str) -> Result<Option<WorkItem>> {
        self.repo.claim_work(worker_id)
    }

    /// Helper para persistir datos del flujo desde capas superiores.
    pub fn persist_flow_data(&self, data: FlowData, expected_version: i64) -> Result<crate::domain::PersistResult> {
        self.engine.persist_data(&data, expected_version)
    }

    /// Leer datos del flujo a partir de un cursor (exclusive).
    pub fn read_data(&self, flow_id: Uuid, from_cursor: i64) -> Result<Vec<FlowData>> {
        self.repo.read_data(&flow_id, from_cursor)
    }
}
