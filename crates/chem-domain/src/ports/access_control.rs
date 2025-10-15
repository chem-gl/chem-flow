//! Puerto para el control de acceso.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{access::AccessorType, DomainError};

#[async_trait]
pub trait AccessControl: Send + Sync {
  async fn grant_molecule_family_access(&self,
                                        family_id: &Uuid,
                                        accessor_id: &Uuid,
                                        accessor_type: AccessorType)
                                        -> Result<(), DomainError>;

  async fn revoke_molecule_family_access(&self,
                                         family_id: &Uuid,
                                         accessor_id: &Uuid,
                                         accessor_type: &AccessorType)
                                         -> Result<(), DomainError>;

  async fn has_molecule_family_access(&self, user_id: &Uuid, family_id: &Uuid) -> Result<bool, DomainError>;

  async fn grant_molecule_access(&self,
                                 molecule_id: &Uuid,
                                 accessor_id: &Uuid,
                                 accessor_type: AccessorType)
                                 -> Result<(), DomainError>;

  async fn revoke_molecule_access(&self,
                                  molecule_id: &Uuid,
                                  accessor_id: &Uuid,
                                  accessor_type: &AccessorType)
                                  -> Result<(), DomainError>;

  async fn has_molecule_access(&self, user_id: &Uuid, molecule_id: &Uuid) -> Result<bool, DomainError>;

  async fn grant_flow_access(&self,
                             flow_id: &Uuid,
                             accessor_id: &Uuid,
                             accessor_type: AccessorType)
                             -> Result<(), DomainError>;

  async fn revoke_flow_access(&self,
                              flow_id: &Uuid,
                              accessor_id: &Uuid,
                              accessor_type: &AccessorType)
                              -> Result<(), DomainError>;

  async fn has_flow_access(&self, user_id: &Uuid, flow_id: &Uuid) -> Result<bool, DomainError>;
}
