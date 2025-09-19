use crate::engine::ChemicalFlowEngine;
use crate::engine::WorkflowConfig;
use crate::workflow_type::WorkflowType;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// Use the in-memory repositories from the workspace for the factory so the
// created engines are usable out-of-the-box in examples and tests.
use chem_domain::InMemoryDomainRepository;
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;

/// Fabrica para crear o cargar instancias de motores de flujo.
///
/// Provee metodos de creacion rapida que usan repositorios en memoria
/// para facilitar ejemplos y tests. Las instancias devueltas implementan
/// `ChemicalFlowEngine` y estan listas para ejecutar pasos localmente.
pub struct ChemicalWorkflowFactory;

impl ChemicalWorkflowFactory {
    /// Crea un nuevo motor para un tipo de workflow identificado por
    /// `workflow_type`.
    ///
    /// Devuelve una instancia lista para usar. Actualmente el camino
    /// por defecto utiliza repositorios en memoria para que los ejemplos
    /// no requieran dependencias externas.
    pub fn create(_workflow_type: &WorkflowType, _config: WorkflowConfig) -> Box<dyn ChemicalFlowEngine> {
        // For now we only provide a simple concrete path for `Cadma` flows
        // using in-memory repositories so callers get a runnable engine.
        match _workflow_type {
            WorkflowType::Cadma => {
                let concrete = InMemoryFlowRepository::new();
                let repo: Arc<dyn FlowRepository> = Arc::new(concrete);
                let id = repo.create_flow(Some("cadma-auto".into()),
                                          Some("created".into()),
                                          json!({"workflow_type": _workflow_type.to_string()}))
                             .expect("failed to create in-memory flow");
                let domain_concrete = InMemoryDomainRepository::default();
                let domain_arc = Arc::new(domain_concrete);
                Box::new(crate::flows::CadmaFlow::new_with_repos(id, _config, repo, domain_arc))
            }
            WorkflowType::Unknown => Self::create(&WorkflowType::Cadma, _config),
        }
    }

    /// Carga una instancia apuntando a un `flow_id` existente.
    ///
    /// Intenta rehidratar el motor con el snapshot existente si la
    /// persistencia lo soporta; en este stub se ignoran errores de
    /// rehidratacion (best-effort).
    pub fn load(_flow_id: &Uuid) -> Box<dyn ChemicalFlowEngine> {
        // Load returns an engine instance pointed at an existing flow id.
        // Use in-memory repos by default so the returned engine is usable in
        // examples. We attempt rehydration but ignore errors (best-effort).
        let concrete = InMemoryFlowRepository::new();
        let repo: Arc<dyn FlowRepository> = Arc::new(concrete);
        let domain_concrete = InMemoryDomainRepository::default();
        let domain_arc = Arc::new(domain_concrete);

        let cfg = WorkflowConfig::default();
        let mut engine = crate::flows::CadmaFlow::new_with_repos(*_flow_id, cfg.clone(), repo, domain_arc);
        // best-effort rehydrate; ignore errors here
        let _ = engine.rehydrate();
        Box::new(engine)
    }
}
