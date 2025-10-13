// molecule_initial_step3.rs
//! Paso 3: Generar molécula inicial.
//! - Soporta métodos Manual y Random.
//! - Manual: usa SMILES proporcionadas.
//! - Random: selecciona de una lista de candidatos configurables.
//! - Guarda las moléculas generadas mediante los ports del dominio.
use crate::errors::WorkflowError;
use crate::step::StepContext;
use chem_domain::ports::ProviderMolecule;
use chem_domain::Molecule;
use chem_providers::{ChemEngine, ChemEngineInterface};
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
    // TODO Phase 4: Inject PropertyProvider via context instead of static ENGINE
    let engine = ChemEngine::init().map_err(|e| {
                                     WorkflowError::Domain(chem_domain::DomainError::provider("ChemEngine",
                                                                            format!("Failed to initialize: {}", e)))
                                   })?;
    for smiles in &smiles_list {
      let provider_mol = engine.get_molecule(smiles).map_err(|e| {
                                                       WorkflowError::Domain(chem_domain::DomainError::provider("ChemEngine",
                                                                                               format!("Failed to parse \
                                                                                                        SMILES: {}",
                                                                                                       e)))
                                                     })?;
      // Convert chem_providers::Molecule to chem_domain::ports::ProviderMolecule
      let domain_provider_mol = ProviderMolecule { inchikey: provider_mol.inchikey,
                                                   inchi: provider_mol.inchi,
                                                   smiles: provider_mol.smiles,
                                                   num_atoms: provider_mol.num_atoms,
                                                   mol_weight: provider_mol.mol_weight,
                                                   mol_formula: provider_mol.mol_formula,
                                                   structure: None /* TODO: convert structure if needed */ };
      let molecule = Molecule::from_provider_molecule(domain_provider_mol).map_err(WorkflowError::Domain)?;
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
