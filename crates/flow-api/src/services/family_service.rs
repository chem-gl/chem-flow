use crate::errors::ApiError;
use crate::models::{CreateFamilyRequest, FamilyResponse};
use chem_domain::FamilyRepository;
use chem_domain::MoleculeReader;
use chem_domain::{Molecule, MoleculeFamily};
use chem_persistence::DieselDomainRepository;
use std::sync::Arc;

pub struct FamilyService {
  repo: Arc<DieselDomainRepository>,
}

impl FamilyService {
  pub fn new(repo: Arc<DieselDomainRepository>) -> Self {
    Self { repo }
  }

  pub async fn create_family(&self, req: CreateFamilyRequest) -> Result<FamilyResponse, ApiError> {
    // Build family from provided molecule inchikeys
    let mut molecules: Vec<Molecule> = Vec::new();
    for ik in &req.molecule_inchikeys {
      if let Ok(Some(m)) = self.repo.get_molecule(ik) {
        molecules.push(m);
      }
    }
    let family = MoleculeFamily::new(molecules, req.provenance).map_err(|e| ApiError::DomainError(e.to_string()))?;
    let id = self.repo.save_family(family.clone()).map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    Ok(FamilyResponse { id,
                        name: family.name().map(|s| s.to_string()),
                        description: family.description().map(|s| s.to_string()),
                        provenance: family.provenance().clone(),
                        molecule_inchikeys: family.molecules().iter().map(|m| m.inchikey().to_string()).collect() })
  }
}
