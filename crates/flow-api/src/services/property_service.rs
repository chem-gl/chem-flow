use crate::errors::ApiError;
use crate::models::CreateMolecularPropertyRequest;
use chem_domain::OwnedMolecularProperty;
use chem_domain::PropertyRepository;
use chem_persistence::DieselDomainRepository;
use std::sync::Arc;

pub struct PropertyService {
  repo: Arc<DieselDomainRepository>,
}

impl PropertyService {
  pub fn new(repo: Arc<DieselDomainRepository>) -> Self {
    Self { repo }
  }

  pub async fn create_molecular_property(&self, _req: CreateMolecularPropertyRequest) -> Result<(), ApiError> {
    // Build OwnedMolecularProperty from request
    let prop = OwnedMolecularProperty { id: uuid::Uuid::new_v4(),
                                        molecule_inchikey: _req.molecule_inchikey.clone(),
                                        property_type: _req.property_type.clone(),
                                        value: _req.value.clone(),
                                        quality: _req.quality.clone(),
                                        preferred: _req.preferred,
                                        value_hash: format!("{}_{}", _req.property_type, uuid::Uuid::new_v4()),
                                        metadata: _req.metadata.clone() };

    // Persist via repository
    self.repo.save_molecular_property(prop).map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    Ok(())
  }

  pub async fn create_molecular_property_with_owner(&self,
                                                    req: CreateMolecularPropertyRequest,
                                                    owner: Option<uuid::Uuid>)
                                                    -> Result<(), ApiError> {
    let prop =
      chem_domain::OwnedMolecularProperty { id: uuid::Uuid::new_v4(),
                                            molecule_inchikey: req.molecule_inchikey.clone(),
                                            property_type: req.property_type.clone(),
                                            value: req.value.clone(),
                                            quality: req.quality.clone(),
                                            preferred: req.preferred,
                                            value_hash: format!("{}_{}", req.property_type, uuid::Uuid::new_v4()),
                                            metadata: req.metadata.clone() };

    // If owner provided, check they have access to the molecule first
    if let Some(uid) = owner {
      let ik = prop.molecule_inchikey.clone();
      if let Ok(Some(m)) = chem_domain::MoleculeReader::get_molecule(&*self.repo, &ik) {
        let has = chem_domain::ports::AccessControl::has_molecule_access(&*self.repo, &uid, &m.id())
          .await
          .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
        if !has {
          return Err(ApiError::Unauthorized("no access to molecule".to_string()));
        }
      }
    }

    chem_domain::PropertyRepository::save_molecular_property(&*self.repo, prop.clone()).map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    if let Some(uid) = owner {
      // Lookup molecule by inchikey to obtain UUID and grant molecule access to owner
      let ik = prop.molecule_inchikey.clone();
      if let Ok(Some(m)) = chem_domain::MoleculeReader::get_molecule(&*self.repo, &ik) {
        let mid = m.id();
        chem_domain::ports::AccessControl::grant_molecule_access(&*self.repo, &mid, &uid, chem_domain::access::AccessorType::User)
          .await
          .map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
      } else {
        // If molecule not found, skip grant silently (could also return an
        // error)
      }
    }
    Ok(())
  }

  pub async fn get_molecular_properties(&self,
                                        inchikey: &str)
                                        -> Result<Vec<crate::models::MolecularPropertyResponse>, ApiError> {
    let props = chem_domain::PropertyRepository::get_molecular_properties(&*self.repo, inchikey).map_err(|e| ApiError::InternalError(format!("DB error: {}", e)))?;
    let out = props.into_iter()
                   .map(|p| crate::models::MolecularPropertyResponse { id: p.id,
                                                                       molecule_inchikey: p.molecule_inchikey,
                                                                       property_type: p.property_type,
                                                                       value: p.value,
                                                                       quality: p.quality,
                                                                       preferred: p.preferred,
                                                                       value_hash: p.value_hash,
                                                                       metadata: p.metadata })
                   .collect();
    Ok(out)
  }
}
