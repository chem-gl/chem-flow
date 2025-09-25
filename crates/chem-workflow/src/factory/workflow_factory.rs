use crate::engine::ChemicalFlowEngine;
use crate::WorkflowError;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
// Usar los repositorios en memoria del workspace para la fábrica para que
// los motores creados sean utilizables de inmediato en ejemplos y pruebas.
use crate::workflow_type::WorkflowType;
use flow::repository::FlowRepository;
use std::collections::HashMap;
// la rehidratación se delega al motor concreto vía `rehydrate`
/// Fábrica para crear o cargar instancias de motores de flujo.
///
/// Provee métodos de creación rápidos que usan repositorios (por defecto
/// los de `chem-persistence`) para facilitar ejemplos y tests. Las
/// instancias devueltas implementan `ChemicalFlowEngine` y están listas
/// para ejecutar pasos localmente.
pub struct ChemicalWorkflowFactory;
impl ChemicalWorkflowFactory {
  /// Lista todos los flows y sus tipos de workflow.
  /// Retorna un `HashMap` desde el UUID (string) del flow hasta su
  /// `WorkflowType`.
  pub fn get_chem_flows() -> Result<HashMap<String, WorkflowType>, WorkflowError> {
    let repo = chem_persistence::new_flow_from_env()?;
    let repo_arc: Arc<dyn FlowRepository> = Arc::new(repo);
    let ids = repo_arc.list_flow_ids()?;
    let mut out = HashMap::new();
    for id in ids {
      let wt = match repo_arc.get_meta(&id, "workflow_type") {
        Ok(v) => v.as_str().and_then(|s| s.parse::<WorkflowType>().ok()).unwrap_or(WorkflowType::Unknown),
        Err(_) => WorkflowType::Unknown,
      };
      out.insert(id.to_string(), wt);
    }
    Ok(out)
  }
  /// Constructor genérico que crea un nuevo flow y construye el engine
  /// concreto `E` asociado.
  ///
  /// - `create_name`: nombre del flow a crear en la persistencia.
  ///
  /// El tipo `E` debe implementar `ChemicalFlowEngine`. El método usa
  /// `E::engine_workflow_type()` para registrar el tipo en los metadatos.
  pub fn create<E>(create_name: String) -> Result<Box<E>, WorkflowError>
    where E: ChemicalFlowEngine + 'static
  {
    let workflow_type = E::engine_workflow_type();
    let repo = chem_persistence::new_flow_from_env()?;
    let repo_arc: Arc<dyn FlowRepository> = Arc::new(repo);
    let id = repo_arc.create_flow(Some(create_name), Some("created".into()), json!({}))?;
    repo_arc.set_meta(&id, "workflow_type", json!(workflow_type.to_string()))?;
    let domain_repo = chem_persistence::new_domain_from_env()?;
    let domain_arc: Arc<dyn chem_domain::DomainRepository> = Arc::new(domain_repo);
    let engine = E::construct_with_repos(id, repo_arc, domain_arc);
    Ok(Box::new(engine))
  }
  /// Carga una instancia apuntando a un `flow_id` existente.
  ///
  /// Intenta rehidratar el motor con el snapshot existente si está
  /// disponible; la rehidratación concreta la realiza la implementación
  /// del engine en `E::rehydrate`.
  pub fn load<E>(_flow_id: &Uuid) -> Result<Box<E>, WorkflowError>
    where E: ChemicalFlowEngine + 'static
  {
    // inicializar repositorios respaldados por persistencia (obligatorio)
    let repo = chem_persistence::new_flow_from_env()?;
    let repo_arc: Arc<dyn FlowRepository> = Arc::new(repo);
    let domain_repo = chem_persistence::new_domain_from_env()?;
    let domain_arc: Arc<dyn chem_domain::DomainRepository> = Arc::new(domain_repo);
    let engine = E::rehydrate(*_flow_id, repo_arc.clone(), domain_arc)?;
    if let Ok(meta_val) = engine.get_metadata("flow_metadata") {
      let has_cs = meta_val.get("current_step").is_some();
      if !has_cs {
        if let Ok(flow_meta) = repo_arc.get_flow_meta(_flow_id) {
          let cs = flow_meta.current_cursor as u32;
          let _ = engine.set_metadata("flow_metadata", json!({ "current_step": cs }));
        }
      }
    }
    Ok(Box::new(engine))
  }
}
