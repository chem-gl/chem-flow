use crate::engine::ChemicalFlowEngine;
use crate::errors::WorkflowError;
use crate::step::{StepContext, StepInfo, WorkflowStep};
use base64::Engine;
use chem_domain::DomainRepository;
use flow::repository::FlowRepository;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

/// Variantes de estado publico expuestas por el flow
#[derive(Debug)]
pub enum FlowStatus {
  /// No iniciado
  NotStarted,
  /// En ejecucion
  Running,
  /// Completado
  Completed,
  /// Fallido
  Failed,
  /// Desconocido
  Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadmaState {
  /// Paso actual (contador logico, 0-based)
  pub current_step: u32,
  /// Referencias a objetos de dominio producidos por pasos previos
  pub domain_refs: Vec<Uuid>,
  /// Metadata general del flujo (parametros, ultimo resultado, etc.)
  pub metadata: JsonValue,
  /// Estado textual del flujo (ej. "not_started", "running", "completed")
  pub status: String,
}

#[derive(Clone)]
pub struct CadmaFlow {
  /// Identificador único del flow en el repositorio.
  pub id: Uuid,
  /// Estado serializable del flujo (pasos actuales, referencias, metadata).
  pub state: CadmaState,
  /// Repositorio para persistir `FlowData`, snapshots y metadatos.
  pub flow_repo: Arc<dyn FlowRepository>,
  /// Repositorio/servicio del dominio químico (moléculas, familias).
  pub domain_repo: Arc<dyn DomainRepository>,
}

impl CadmaFlow {
  /// Constructor que toma repositorios de flujo y dominio
  pub fn new(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self {
    let state = CadmaState { current_step: 0,
                             domain_refs: Vec::new(),
                             metadata: JsonValue::Object(serde_json::Map::new()),
                             status: "not_started".to_string() };
    Self { id, state, flow_repo, domain_repo }
  }

  /// Constructor alternativo usado por tests donde los
  /// repositorios se proveen después de crear la fila del flow.
  pub fn new_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self {
    Self::new(id, flow_repo, domain_repo)
  }

  /// Devuelve el índice del paso actual (contador lógico, 0-based).
  pub fn current_step(&self) -> u32 {
    self.state.current_step
  }

  /// Estado ligero del flujo para consumidores.
  pub fn status(&self) -> FlowStatus {
    match self.state.status.as_str() {
      "not_started" => FlowStatus::NotStarted,
      "running" => FlowStatus::Running,
      "completed" => FlowStatus::Completed,
      "failed" => FlowStatus::Failed,
      _ => FlowStatus::Unknown,
    }
  }

  /// Guarda un snapshot del estado actual en el repositorio.
  ///
  /// Serializa `CadmaState` a bytes y guarda un puntero (base64) en la
  /// tabla de snapshots. Método placeholder - la estrategia de
  /// almacenamiento puede cambiar.
  pub fn save_snapshot(&self) -> Result<(), WorkflowError> {
    // Serializar estado a bytes y llamar al repositorio para guardar
    let state_bytes = serde_json::to_vec(&self.state)?;
    // Para simplicidad almacenamos los bytes como base64 en el campo
    // `state_ptr` de la metadata del snapshot; el SnapshotStore puede
    // decodificarlo cuando sea necesario.
    let state_ptr = base64::engine::general_purpose::STANDARD.encode(&state_bytes);
    let _snapshot_id = self.flow_repo.save_snapshot(&self.id,
                                                     self.state.current_step as i64,
                                                     &state_ptr,
                                                     self.state.metadata.clone())?;
    Ok(())
  }

  /// Rehidrata el estado del motor usando el último snapshot disponible.
  ///
  /// Intenta cargar la metadata del último snapshot y decodificar el
  /// `state_ptr` (base64) para reconstruir `CadmaState`.
  pub fn rehydrate(&mut self) -> Result<(), WorkflowError> {
    // Intentar cargar metadata del último snapshot
    if let Some(snapshot_meta) = self.flow_repo.load_latest_snapshot(&self.id)? {
      // En la implementación en memoria/state_ptr es una cadena base64
      let state_ptr = snapshot_meta.state_ptr;
      // Decodificar base64 a bytes
      let bytes = base64::engine::general_purpose::STANDARD
                .decode(state_ptr.as_bytes())
                .map_err(|e| WorkflowError::Persistence(format!("error al decodificar base64: {}", e)))?;
      // Deserializar CadmaState
      // Nota: aquí se recupera el estado completo del engine desde el
      // snapshot. Si no hay snapshot, no se realiza replay de eventos.
      let state: CadmaState = serde_json::from_slice(&bytes)?;
      self.state = state;
      return Ok(());
    }

    // Si no hay snapshot válido no hacemos nada por ahora. Opcionalmente
    // se podría rehidatar mediante replay de `FlowData` desde el repo.
    Ok(())
  }

  /// Crea una rama desde un cursor dado
  pub fn create_branch(&self,
                       parent_cursor: i64,
                       name: Option<String>,
                       status: Option<String>)
                       -> Result<Uuid, WorkflowError> {
    let branch_id = self.flow_repo.create_branch(&self.id, name, status, parent_cursor, JsonValue::Null)?;
    Ok(branch_id)
  }

  /// Limpia pasos desde un cursor (prune) - placeholder, asume metodo no
  /// existe
  pub fn clean_from_step(&self, _cursor: i64) -> Result<(), WorkflowError> {
    // Placeholder: implementar si el repositorio soporta borrado por cursor
    Err(WorkflowError::Persistence("eliminar_pasos_desde_cursor no implementado".to_string()))
  }

  fn get_current_step(&self) -> Result<Box<dyn WorkflowStep>, WorkflowError> {
    match self.state.current_step {
      0 => Ok(Box::new(super::steps::Step1)),
      1 => Ok(Box::new(super::steps::Step2)),
      2 => Ok(Box::new(super::steps::Step3)),
      _ => Err(WorkflowError::Validation("No hay más pasos".to_string())),
    }
  }

  // Helper: asegurar que todos los pasos previos requeridos tengan payloads
  // persistidos. Centraliza la lógica que estaba duplicada y mantiene las
  // rutas de ejecución más simples y legibles.
  fn ensure_previous_steps_present(&self, required: &[String]) -> Result<(), WorkflowError> {
    let mut missing = Vec::new();
    for req in required {
      if self.get_last_step_payload(req)?.is_none() {
        missing.push(req.clone());
      }
    }
    if !missing.is_empty() {
      return Err(WorkflowError::Validation(format!("Missing data from previous steps: {:?}", missing)));
    }
    Ok(())
  }

  // Helper: buscar el último payload para una clave dada. Extraído para que
  // los llamadores no necesiten conocer la forma en que se itera `FlowData`.
  // Retorna Ok(None) si no se encuentra payload.
  fn find_last_payload_by_key(&self, key: &str) -> Result<Option<JsonValue>, WorkflowError> {
    let data = self.flow_repo.read_data(&self.id, 0)?;
    for fd in data.iter().rev() {
      if fd.key == key {
        return Ok(Some(fd.payload.clone()));
      }
    }
    Ok(None)
  }

  /// Devuelve el nombre del paso actual como string (útil para logging).
  /// Se deriva de `get_current_step()` y permite obtener el id del paso
  /// sin necesidad de instanciar la lógica del paso.
  pub fn current_step_name(&self) -> Result<String, WorkflowError> {
    let s = self.get_current_step()?;
    Ok(s.name().to_string())
  }
}

impl ChemicalFlowEngine for CadmaFlow {
  fn id(&self) -> Uuid {
    self.id
  }

  fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>> {
    self.state = serde_json::from_value(snapshot.clone())?;
    Ok(())
  }

  fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>> {
    Ok(serde_json::to_value(&self.state)?)
  }

  fn engine_workflow_type() -> crate::workflow_type::WorkflowType
    where Self: Sized
  {
    crate::workflow_type::WorkflowType::Cadma
  }

  fn create_branch(&self, parent_cursor: i64, name: Option<String>, status: Option<String>) -> Result<Uuid, Box<dyn Error>> {
    Ok(self.create_branch(parent_cursor, name, status)?)
  }

  fn construct_with_repos(id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self
    where Self: Sized
  {
    Self::new_with_repos(id, flow_repo, domain_repo)
  }

  fn rehydrate_with_repos(id: Uuid,
                          flow_repo: Arc<dyn FlowRepository>,
                          domain_repo: Arc<dyn DomainRepository>)
                          -> Result<Self, crate::errors::WorkflowError>
    where Self: Sized
  {
    // Construir engine
    let mut engine = Self::new_with_repos(id, flow_repo.clone(), domain_repo);
    // Intentar cargar y aplicar el último snapshot si existe
    if let Some(snapshot_meta) = engine.flow_repo.load_latest_snapshot(&engine.id)? {
      let (bytes, _meta) = engine.flow_repo.load_snapshot(&snapshot_meta.id)?;
      if let Ok(state_ptr_b64) = String::from_utf8(bytes) {
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(state_ptr_b64.as_bytes()) {
          let state: CadmaState = serde_json::from_slice(&decoded)?;
          engine.state = state;
        }
      }
    } else {
      // Si no había snapshot, sincronizar estado ligero desde el repo.
      // Esto es importante para ramas creadas mediante `create_branch`:
      // el repo copia los `FlowData` hasta `parent_cursor` y ajusta
      // `FlowMeta.current_cursor`. Si no sincronizamos, el engine
      // arrancará en el paso 0 aunque ya existan pasos persistidos.
      //
      // Strategy: usar `count_steps` para determinar cuántos pasos
      // efectivos existen y usar ese número como `current_step` (0-based
      // índice del siguiente paso a ejecutar). Además, copiar metadata
      // y status desde `FlowMeta` para consistencia.
      if let Ok(meta) = engine.flow_repo.get_flow_meta(&engine.id) {
        // Usar `FlowMeta.current_cursor` como fuente de verdad para el
        // estado ligero del engine. `current_cursor` indica el último
        // cursor persistido; el siguiente paso a ejecutar coincide
        // con ese valor (índice 0-based del siguiente paso).
        engine.state.current_step = meta.current_cursor as u32;
        engine.state.metadata = meta.metadata.clone();
        engine.state.status = meta.status.unwrap_or_else(|| "unknown".to_string());
      }
    }

    Ok(engine)
  }
  fn flow_repo(&self) -> &Arc<dyn FlowRepository> {
    &self.flow_repo
  }
}

impl CadmaFlow {
  /// Ejecuta el paso actual (segun `state.current_step`) y devuelve el
  /// `StepInfo`. No persiste automáticamente: devuelve el resultado para
  /// que el caller decida cuándo persistirlo.
  pub fn execute_current_step(&mut self, input: &JsonValue) -> Result<StepInfo, WorkflowError> {
    let step = self.get_current_step()?;
    // Validate required previous steps have data persisted using helper.
    let required = step.required_previous_steps();
    self.ensure_previous_steps_present(&required)?;
    // Cada step implementa un método `execute(&self, input: &JsonValue)` o
    // `execute_with_context(&self, ctx, input)` que devuelve `StepInfo`.
    // NOTAS EXPLICITAS (ES):
    // - El valor devuelto (StepInfo) contiene `payload` y `metadata`. El `payload`
    //   normalmente contiene el DTO resultante del paso (ej. `Step2Payload`) y
    //   `metadata` contiene status/params.
    // - Para persistir, el caller debe usar `persist_step_result`, que empaqueta
    //   `StepInfo.payload` y `StepInfo.metadata` en un `FlowData` con key
    //   `step_state:{step}` (ver abajo).
    // - Las funciones que regresan el valor del paso son `execute`, `execute_typed`
    //   (retorna DTO tipado) y `execute_with_context` (retorna `StepInfo`).
    // Build a StepContext so steps can access typed helpers and repos.
    let ctx = StepContext::new(self.id, self.flow_repo.clone(), self.domain_repo.clone());
    // Dispatch generically via the `WorkflowStep` trait `execute` method.
    step.execute(&ctx, input)
  }

  /// Ejecuta un step por indice (0-based), util para ejecutarlo de forma
  /// manual sin depender del estado interno.
  pub fn execute_step_by_index(&self, index: u32, input: &JsonValue) -> Result<StepInfo, WorkflowError> {
    // Use helper to validate required previous steps for each concrete
    // step and keep linear control flow.
    match index {
      0 => {
        let s = super::steps::Step1 {};
        self.ensure_previous_steps_present(&s.required_previous_steps())?;
        let ctx = StepContext::new(self.id, self.flow_repo.clone(), self.domain_repo.clone());
        Ok(s.execute(&ctx, input)?)
      }
      1 => {
        let s = super::steps::Step2 {};
        self.ensure_previous_steps_present(&s.required_previous_steps())?;
        let ctx = StepContext::new(self.id, self.flow_repo.clone(), self.domain_repo.clone());
        Ok(s.execute(&ctx, input)?)
      }
      2 => {
        let s = super::steps::Step3 {};
        self.ensure_previous_steps_present(&s.required_previous_steps())?;
        let ctx = StepContext::new(self.id, self.flow_repo.clone(), self.domain_repo.clone());
        Ok(s.execute(&ctx, input)?)
      }
      _ => Err(WorkflowError::Validation("Step index out of range".to_string())),
    }
  }

  /// Lee el ultimo payload persistido para un step dado (key
  /// `step_state:{step}`)
  pub fn get_last_step_payload(&self, step_name: &str) -> Result<Option<JsonValue>, WorkflowError> {
    let key = format!("step_state:{}", step_name);
    // Delegate to iterator helper.
    self.find_last_payload_by_key(&key)
  }

  /// Persiste el resultado de un paso como `FlowData` usando la convención
  /// de clave `step_state:{step_name}`. `expected_version` es pasado al repo
  /// para control de concurrencia. `command_id` permite idempotencia.
  /// Persistir resultado tipado de paso.
  ///
  /// Convenciones:
  /// - key: "step_state:{step_name}".
  /// - payload: el DTO serializado (usado por otros pasos con
  ///   `get_last_step_payload` o con helpers tipados).
  /// - metadata: metadatos operativos (status, params, domain_refs, etc.).
  /// - command_id: opcional para idempotencia (si se repite la misma operación
  ///   no se duplicará).
  ///
  /// Si `expected_version` es negativo se intentará obtener `FlowMeta` y
  /// usar `current_version` para la llamada; además se calculará un cursor
  /// candidato `current_cursor + 1`. Esto cubre implementaciones de repo
  /// que esperan cursor/version explícitos. Si el repo maneja cursor
  /// internamente la implementación del repo puede ignorar `cursor`.
  pub fn persist_step_result(&self,
                             step_name: &str,
                             info: StepInfo,
                             expected_version: i64,
                             command_id: Option<Uuid>)
                             -> Result<flow::domain::PersistResult, WorkflowError> {
    use chrono::Utc;
    use flow::domain::FlowData;

    let key = format!("step_state:{}", step_name);

    // Calcular candidato de cursor siempre desde el estado del repo
    // para asegurar monotonía y coherencia entre ramas/recargas.
    //
    // Regla: el siguiente cursor a asignar debe ser `FlowMeta.current_cursor + 1`.
    // Si el caller no proporcionó `expected_version` (valor < 0), se
    // usa `FlowMeta.current_version` como `expected_version` para la
    // llamada al repositorio; si el caller proporcionó un `expected_version`
    // explícito se respetará (esto permite control optimista desde afuera).
    let mut cursor_candidate: i64 = 0;
    let mut ev = expected_version;
    if let Ok(meta) = self.flow_repo.get_flow_meta(&self.id) {
      cursor_candidate = meta.current_cursor + 1;
      if expected_version < 0 {
        ev = meta.current_version;
      }
    }

    // Construir FlowData con cursor candidato calculado arriba. El
    // repositorio debe validar la monotonía del cursor y actualizar
    // `FlowMeta.current_cursor` y `current_version` atomically cuando
    // persista el registro.
    let data = FlowData { id: Uuid::new_v4(),
                          flow_id: self.id,
                          cursor: cursor_candidate,
                          key,
                          payload: info.payload,
                          metadata: info.metadata,
                          command_id,
                          created_at: Utc::now() };

    // Llamada al repositorio para persistir.
    match self.flow_repo.persist_data(&data, ev)? {
      flow::domain::PersistResult::Ok { new_version } => Ok(flow::domain::PersistResult::Ok { new_version }),
      flow::domain::PersistResult::Conflict => Ok(flow::domain::PersistResult::Conflict),
    }
  }

  /// Avanza el contador de steps en memoria (no persiste snapshot). Usar
  /// junto a `persist_step_result` para marcar avance del flujo.
  pub fn advance_step(&mut self) {
    self.state.current_step = self.state.current_step.saturating_add(1);
  }
}
