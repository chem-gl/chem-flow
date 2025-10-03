// molecule_initial_step3.rs
//! Paso 3: Generar molécula inicial.
//! - Soporta métodos Manual y Random.
//! - Manual: usa SMILES proporcionadas.
//! - Random: selecciona de una lista de candidatos configurables.
//! - Guarda las moléculas generadas en domain_repo.

use crate::errors::WorkflowError;
use crate::step::StepContext;
use chem_domain::Molecule;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Input {
  pub method: GenerationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationMethod {
  Manual { smiles: String },
  Random { candidates: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Payload {
  pub generated_molecules: Vec<String>, // inchikeys
  pub method_used: String,
  pub step_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Metadata {
  pub status: String,
  pub parameters: Step3Params,
  pub domain_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Params {
  pub method: GenerationMethod,
}

#[derive(Debug, Default, Clone)]
pub struct MoleculeInitialStep3;

impl MoleculeInitialStep3 {
  /// Ejecuta el step: genera moléculas según el método y las guarda.
  pub fn execute_step(&self, ctx: &StepContext, input: Step3Input) -> Result<crate::step::StepInfo, WorkflowError> {
    let smiles_list = match &input.method {
      GenerationMethod::Manual { smiles } => vec![smiles.clone()],
      GenerationMethod::Random { candidates } => {
        // Para random, seleccionar una o todas? Digamos todas por ahora.
        candidates.clone()
      }
    };

    let mut generated_inchikeys = Vec::new();
    let mut domain_refs = Vec::new();

    for smiles in &smiles_list {
      let molecule = Molecule::from_smiles(smiles).map_err(WorkflowError::Domain)?;
      let inchikey = ctx.domain_repo.save_molecule(molecule.clone())?;
      generated_inchikeys.push(inchikey.clone());
      domain_refs.push(inchikey);
    }

    let method_str = match &input.method {
      GenerationMethod::Manual { .. } => "Manual".to_string(),
      GenerationMethod::Random { .. } => "Random".to_string(),
    };

    let payload = Step3Payload { generated_molecules: generated_inchikeys.clone(),
                                 method_used: method_str.clone(),
                                 step_result: format!("Generadas {} moléculas usando método {}",
                                                      generated_inchikeys.len(),
                                                      method_str) };

    let metadata =
      Step3Metadata { status: "completed".to_string(), parameters: Step3Params { method: input.method }, domain_refs };

    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }
}

crate::impl_workflow_step!(MoleculeInitialStep3,
                           Step3Payload,
                           Step3Metadata,
                           Step3Input,
                           |this_self, ctx, input| { this_self.execute_step(ctx, input) });
