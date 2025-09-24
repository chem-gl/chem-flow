use crate::errors::WorkflowError;
use crate::step::StepInfo;
use chem_domain::DomainRepository;
use flow::repository::FlowRepository;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use uuid::Uuid;

/// Contexto pasado a pasos para facilitar acceso tipado a resultados
/// previos y persistencia de resultados tipados.
///
/// Este helper no fuerza acoplamiento con toda la estructura `CadmaFlow` y
/// puede ser usado por implementaciones de pasos que necesiten leer
/// resultados de pasos previos en forma tipada.
pub struct StepContext {
  /// Identificador del flujo al que pertenece el contexto.
  pub flow_id: Uuid,
  /// Repositorio para leer/guardar `FlowData` y snapshots.
  pub flow_repo: Arc<dyn FlowRepository>,
  /// Repositorio del dominio químico (moléculas, familias, etc.).
  pub domain_repo: Arc<dyn DomainRepository>,
}

impl StepContext {
  /// Crea un nuevo contexto para el flow indicado.
  pub fn new(flow_id: Uuid, flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<dyn DomainRepository>) -> Self {
    Self { flow_id, flow_repo, domain_repo }
  }

  /// Obtiene el ultimo payload persistido para `step_name` y lo deserializa
  /// en T. Retorna Ok(None) si no existe dato previo.
  pub fn get_typed_output<T: DeserializeOwned>(&self, step_name: &str) -> Result<Option<T>, WorkflowError> {
    // Aquí se busca en la tabla lógica de `FlowData` (usando
    // FlowRepository::read_data) la última entrada cuya key sea
    // `step_state:{step_name}`. El `payload` almacenado es el DTO
    // serializado (por ejemplo `Step2Payload`) y `metadata` contiene los
    // metadatos asociados. Para rehidratar en tipos fuertes se deserializa
    // el `payload` a `T` usando `serde_json::from_value`.
    let key = format!("step_state:{}", step_name);
    let data = self.flow_repo.read_data(&self.flow_id, 0)?;
    for fd in data.iter().rev() {
      if fd.key == key {
        let t: T = serde_json::from_value(fd.payload.clone())?;
        return Ok(Some(t));
      }
    }
    Ok(None)
  }

  /// Guarda un `StepInfo` como `step_state:{step_name}` usando convenciones
  /// del engine. `expected_version` puede ser -1 para indicar que se
  /// tome el valor actual del `FlowMeta`.
  pub fn save_typed_result(&self,
                           step_name: &str,
                           info: StepInfo,
                           expected_version: i64,
                           command_id: Option<Uuid>)
                           -> Result<flow::domain::PersistResult, WorkflowError> {
    // Reuse the same logic: build FlowData and call persist_data
    use chrono::Utc;
    use flow::domain::FlowData;

    let key = format!("step_state:{}", step_name);

    // Calcular candidato de cursor siempre desde el estado del repo
    // para mantener coherencia entre ramas y recargas. El valor elegido
    // es `FlowMeta.current_cursor + 1`. Si no se encuentra metadata se
    // deja en 0 y el repositorio podrá decidir la estrategia.
    let mut cursor_candidate: i64 = 0;
    let mut ev = expected_version;
    if let Ok(meta) = self.flow_repo.get_flow_meta(&self.flow_id) {
      cursor_candidate = meta.current_cursor + 1;
      if expected_version < 0 {
        ev = meta.current_version;
      }
    }

    // COMENTARIO EXPLICITO (ES):
    // Aquí se construye la estructura `FlowData` que representa el
    // registro persistente. Campos relevantes:
    // - `key`: se usa la convención `step_state:{step_name}` para poder localizar
    //   los resultados de este paso posteriormente.
    // - `payload`: contiene el DTO serializado (ej. Step2Payload) y es lo que otros
    //   pasos leerán para rehidratar datos tipados.
    // - `metadata`: contiene los metadatos operativos del paso (status, parameters,
    //   domain_refs) y se guarda junto al payload.
    // - `command_id`: opcional para idempotencia.
    // - `cursor`/`created_at`: usados por la persistencia para ordenar y versionar
    //   los registros.
    let data = FlowData { id: Uuid::new_v4(),
                          flow_id: self.flow_id,
                          cursor: cursor_candidate,
                          key,
                          payload: info.payload,
                          metadata: info.metadata,
                          command_id,
                          created_at: Utc::now() };

    match self.flow_repo.persist_data(&data, ev)? {
      flow::domain::PersistResult::Ok { new_version } => Ok(flow::domain::PersistResult::Ok { new_version }),
      flow::domain::PersistResult::Conflict => Ok(flow::domain::PersistResult::Conflict),
    }
  }
}
