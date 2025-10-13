//! Servicio de negocio para ejecutar workflows CADMA
//!
//! Implementa la lógica de orquestación del workflow con persistencia real

use crate::errors::ApiError;
use crate::models::*;
use chem_domain::ports::ProviderMolecule;
use chem_domain::Molecule;
use chem_persistence::DieselDomainRepository;
use chem_providers::{ChemEngine, ChemEngineInterface};
use chem_workflow::engine::chemical_flow::FlowStatus;
use chem_workflow::engine::ChemicalFlowEngine;
use chem_workflow::flows::cadma_flow::steps::common::ADMETSAMethod;
use chem_workflow::flows::cadma_flow::CadmaFlow;
use flow::repository::FlowRepository;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

/// Servicio principal para gestionar ejecuciones de CADMA
pub struct CadmaService {
  flow_repo: Arc<dyn FlowRepository>,
  domain_repo: Arc<DieselDomainRepository>,
}

impl CadmaService {
  /// Crea una nueva instancia del servicio
  pub fn new(flow_repo: Arc<dyn FlowRepository>, domain_repo: Arc<DieselDomainRepository>) -> Self {
    Self { flow_repo, domain_repo }
  }

  /// Inicia una nueva ejecución de CADMA
  pub fn start_execution(&self, req: StartCadmaRequest) -> Result<StartCadmaResponse, ApiError> {
    let name = req.name.unwrap_or_else(|| "cadma-api".to_string());

    // Crear flow en el repositorio
    let flow_id = self.flow_repo.create_flow(Some(name), Some("running".to_string()), req.metadata)?;

    Ok(StartCadmaResponse { execution_id: flow_id,
                            status: "running".to_string(),
                            current_step: 0,
                            created_at: chrono::Utc::now().to_rfc3339() })
  }

  /// Obtiene el estado de una ejecución
  pub fn get_execution_status(&self, execution_id: Uuid) -> Result<CadmaExecutionStatus, ApiError> {
    // Verificar que el flow existe
    let meta = self.flow_repo
                   .get_flow_meta(&execution_id)
                   .map_err(|_| ApiError::NotFound(format!("Ejecución {} no encontrada", execution_id)))?;

    // Construir engine para obtener estado actual
    let engine = CadmaFlow::construct_with_repos(execution_id, self.flow_repo.clone(), self.domain_repo.clone());

    let current_step = engine.current_step() as u8;
    let status = match engine.status() {
      FlowStatus::NotStarted => "not_started",
      FlowStatus::Running => "running",
      FlowStatus::Completed => "completed",
      FlowStatus::Failed => "failed",
      _ => "unknown",
    };

    // Obtener pasos completados
    let steps_completed = self.get_completed_steps(&engine)?;

    // Obtener nombre del paso actual
    let current_step_name = engine.current_step_name().ok();

    Ok(CadmaExecutionStatus { execution_id,
                              status: status.to_string(),
                              current_step,
                              current_step_name,
                              steps_completed,
                              metadata: meta.metadata,
                              updated_at: chrono::Utc::now().to_rfc3339() })
  }

  /// Ejecuta un paso específico del workflow
  pub fn execute_step(&self,
                      execution_id: Uuid,
                      step_index: usize,
                      payload: JsonValue)
                      -> Result<ExecuteStepResponse, ApiError> {
    // Verificar que existe
    self.flow_repo
        .get_flow_meta(&execution_id)
        .map_err(|_| ApiError::NotFound(format!("Ejecución {} no encontrada", execution_id)))?;

    // Construir engine
    let mut engine = CadmaFlow::construct_with_repos(execution_id, self.flow_repo.clone(), self.domain_repo.clone());

    // Validar índice de paso
    if step_index > 5 {
      return Err(ApiError::BadRequest(format!("Índice de paso {} inválido. Debe estar entre 0 y 5", step_index)));
    }

    // Obtener nombre del paso
    let step_name = engine.step_name_by_index(step_index as u32).map_err(|e| ApiError::WorkflowError(e.to_string()))?;

    // Ejecutar paso (sin validar orden estricto)
    let step_info = engine.execute_step_by_index_unchecked(step_index as u32, &payload)
                          .map_err(|e| ApiError::WorkflowError(format!("Error ejecutando paso {}: {}", step_index, e)))?;

    // Persistir resultado
    engine.persist_step_result(&step_name, step_info.clone(), -1, None)
          .map_err(|e| ApiError::PersistenceError(format!("Error persistiendo resultado: {}", e)))?;

    let current_step = engine.current_step() as u8;
    let status = match engine.status() {
      FlowStatus::Running => "running",
      FlowStatus::Completed => "completed",
      FlowStatus::Failed => "failed",
      _ => "running",
    };

    Ok(ExecuteStepResponse { execution_id,
                             step_index,
                             step_name: step_name.clone(),
                             result: step_info.payload,
                             status: status.to_string(),
                             current_step })
  }

  /// Cancela y elimina una ejecución
  pub fn cancel_execution(&self, execution_id: Uuid) -> Result<CancelExecutionResponse, ApiError> {
    // Verificar que existe
    self.flow_repo
        .get_flow_meta(&execution_id)
        .map_err(|_| ApiError::NotFound(format!("Ejecución {} no encontrada", execution_id)))?;

    // TODO: Implementar lógica de cancelación/limpieza real
    // Por ahora, solo confirmamos

    Ok(CancelExecutionResponse { execution_id,
                                 message: format!("Ejecución {} cancelada", execution_id),
                                 cancelled_at: chrono::Utc::now().to_rfc3339() })
  }

  /// Lista todas las ejecuciones
  pub fn list_executions(&self) -> Result<ListExecutionsResponse, ApiError> {
    let flow_ids = self.flow_repo.list_flow_ids()?;

    let mut executions = Vec::new();
    for flow_id in flow_ids {
      if let Ok(meta) = self.flow_repo.get_flow_meta(&flow_id) {
        let engine = CadmaFlow::construct_with_repos(flow_id, self.flow_repo.clone(), self.domain_repo.clone());

        let status = match engine.status() {
          FlowStatus::Running => "running",
          FlowStatus::Completed => "completed",
          FlowStatus::Failed => "failed",
          _ => "unknown",
        };

        executions.push(ExecutionSummary { execution_id: flow_id,
                                           name: meta.name,
                                           status: status.to_string(),
                                           current_step: engine.current_step() as u8,
                                           created_at: meta.created_at.to_rfc3339(),
                                           updated_at: chrono::Utc::now().to_rfc3339() });
      }
    }

    Ok(ListExecutionsResponse { total: executions.len(), executions })
  }

  /// Auxiliar: obtiene pasos completados del engine
  fn get_completed_steps(&self, engine: &CadmaFlow) -> Result<Vec<StepInfo>, ApiError> {
    let mut steps = Vec::new();

    for idx in 0..6u32 {
      if let Ok(step_name) = engine.step_name_by_index(idx) {
        if let Ok(Some(payload)) = engine.get_last_step_payload(&step_name) {
          steps.push(StepInfo { index: idx as usize,
                                name: step_name,
                                output: payload,
                                executed_at: chrono::Utc::now().to_rfc3339() });
        }
      }
    }

    Ok(steps)
  }

  /// Auxiliar: crea molécula desde SMILES
  pub fn molecule_from_smiles(smiles: &str) -> Result<Molecule, ApiError> {
    let engine = ChemEngine::init().map_err(|e| ApiError::InternalError(format!("Error inicializando ChemEngine: {}", e)))?;

    let provider_mol =
      engine.get_molecule(smiles).map_err(|e| ApiError::BadRequest(format!("SMILES inválido '{}': {}", smiles, e)))?;

    let converted = ProviderMolecule { inchikey: provider_mol.inchikey,
                                       inchi: provider_mol.inchi,
                                       smiles: provider_mol.smiles.clone(),
                                       num_atoms: provider_mol.num_atoms,
                                       mol_weight: provider_mol.mol_weight,
                                       mol_formula: provider_mol.mol_formula,
                                       structure: None };

    Ok(Molecule::from_provider_molecule(converted)?)
  }

  /// Convierte string de método a enum ADMETSAMethod
  pub fn parse_admetsa_method(method_str: &str) -> Result<ADMETSAMethod, ApiError> {
    match method_str {
      "Manual" => Ok(ADMETSAMethod::Manual),
      "Random1" => Ok(ADMETSAMethod::Random1),
      "Random2" => Ok(ADMETSAMethod::Random2),
      "Random3" => Ok(ADMETSAMethod::Random3),
      "Random4" => Ok(ADMETSAMethod::Random4),
      _ => Err(ApiError::BadRequest(format!("Método ADMETSA desconocido: {}", method_str))),
    }
  }
}
