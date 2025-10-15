use crate::errors::ApiError;
use crate::models::{CreateMoleculeRequest, MoleculeResponse};
use chem_domain::ports::ProviderMolecule;
use chem_domain::Molecule as DomainMolecule;
use chem_persistence::DieselDomainRepository;
use chem_providers::{ChemEngine, ChemEngineInterface};
use std::sync::Arc;

pub struct MoleculeService {
  repo: Arc<DieselDomainRepository>,
}

impl MoleculeService {
  pub fn new(repo: Arc<DieselDomainRepository>) -> Self {
    Self { repo }
  }

  pub async fn create_molecule(&self, _req: CreateMoleculeRequest) -> Result<MoleculeResponse, ApiError> {
    // Validate and convert SMILES via the chemical engine
    let engine = ChemEngine::init().map_err(|e| ApiError::InternalError(format!("ChemEngine init error: {}", e)))?;
    let provider_mol =
      ChemEngineInterface::get_molecule(&engine, &_req.smiles).map_err(|e| {
                                                                ApiError::BadRequest(format!("SMILES inválido '{}': {}",
                                                                                             _req.smiles, e))
                                                              })?;

    // Convert provider molecule to domain ProviderMolecule
    let converted = ProviderMolecule { inchikey: provider_mol.inchikey,
                                       inchi: provider_mol.inchi,
                                       smiles: provider_mol.smiles.clone(),
                                       num_atoms: provider_mol.num_atoms,
                                       mol_weight: provider_mol.mol_weight,
                                       mol_formula: provider_mol.mol_formula,
                                       structure: None };

    // Build domain Molecule
    let mut mol = DomainMolecule::from_provider_molecule(converted).map_err(|e| ApiError::DomainError(e.to_string()))?;

    // Apply provided metadata (overrides/merges can be added later)
    mol = mol.with_metadata(_req.metadata).map_err(|e| ApiError::DomainError(e.to_string()))?;

    // Persist molecule
    chem_domain::MoleculeWriter::save_molecule(&*self.repo, mol.clone()).map_err(|e| {
                                                                          ApiError::InternalError(format!("DB error: {}", e))
                                                                        })?;

    // Map to response DTO
    Ok(MoleculeResponse { id: mol.id(),
                          inchikey: mol.inchikey().to_string(),
                          smiles: mol.smiles().to_string(),
                          inchi: mol.inchi().to_string(),
                          molecular_formula: mol.molecular_formula().map(|f| f.to_string()),
                          metadata: mol.metadata().clone(),
                          created_at: mol.created_at().to_rfc3339(),
                          updated_at: mol.updated_at().to_rfc3339() })
  }
  pub async fn create_molecule_with_owner(&self,
                                          req: CreateMoleculeRequest,
                                          owner: Option<uuid::Uuid>)
                                          -> Result<MoleculeResponse, ApiError> {
    // Delegate to create_molecule then grant access if owner provided
    let resp = self.create_molecule(req).await?;
    if let Some(uid) = owner {
      // Grant molecule access
      let mid = resp.id;
      chem_domain::ports::AccessControl::grant_molecule_access(&*self.repo, &mid, &uid, chem_domain::access::AccessorType::User)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    }
    Ok(resp)
  }

  pub async fn list_molecules(&self) -> Result<Vec<MoleculeResponse>, ApiError> {
    // Use the repository to list molecules and map to DTOs
    let mols = chem_domain::MoleculeReader::list_molecules(&*self.repo).map_err(|e| {
                                                                         ApiError::InternalError(format!("DB error: {}", e))
                                                                       })?;
    let out = mols.into_iter()
                  .map(|m| MoleculeResponse { id: m.id(),
                                              inchikey: m.inchikey().to_string(),
                                              smiles: m.smiles().to_string(),
                                              inchi: m.inchi().to_string(),
                                              molecular_formula: m.molecular_formula().map(|f| f.to_string()),
                                              metadata: m.metadata().clone(),
                                              created_at: m.created_at().to_rfc3339(),
                                              updated_at: m.updated_at().to_rfc3339() })
                  .collect();
    Ok(out)
  }
}
