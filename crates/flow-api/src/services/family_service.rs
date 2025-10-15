use crate::errors::ApiError;
use crate::models::{CreateFamilyRequest, FamilyResponse};
use chem_domain::access::AccessorType;
use chem_domain::ports::AccessControl;
use chem_domain::FamilyRepository;
use chem_domain::{Molecule, MoleculeFamily};
use chem_persistence::DieselDomainRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct FamilyService {
  repo: Arc<DieselDomainRepository>,
}

impl FamilyService {
  pub fn new(repo: Arc<DieselDomainRepository>) -> Self {
    Self { repo }
  }

  pub async fn create_family(&self, req: CreateFamilyRequest, owner: Option<Uuid>) -> Result<FamilyResponse, ApiError> {
    // Build family from provided molecule inchikeys
    let mut molecules: Vec<Molecule> = Vec::new();
    for ik in &req.molecule_inchikeys {
      // If molecule exists in DB, verify creator has access (if owner provided)
      if let Ok(Some(m)) = chem_domain::MoleculeReader::get_molecule(&*self.repo, ik) {
        if let Some(uid) = owner {
          let has = chem_domain::ports::AccessControl::has_molecule_access(&*self.repo, &uid, &m.id())
            .await
            .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
          if !has {
            return Err(ApiError::Unauthorized("no access to one or more molecules in the family".to_string()));
          }
        }
        molecules.push(m);
      }
    }
    let family = MoleculeFamily::new(molecules, req.provenance).map_err(|e| ApiError::DomainError(e.to_string()))?;
    let id = self.repo.save_family(family.clone()).map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    // Grant access to owner if provided
    if let Some(uid) = owner {
      // grant asynchronously via the AccessControl port
      AccessControl::grant_molecule_family_access(&*self.repo, &id, &uid, AccessorType::User)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    }
    Ok(FamilyResponse { id,
                        name: family.name().map(|s| s.to_string()),
                        description: family.description().map(|s| s.to_string()),
                        provenance: family.provenance().clone(),
                        molecule_inchikeys: family.molecules().iter().map(|m| m.inchikey().to_string()).collect() })
  }
}
