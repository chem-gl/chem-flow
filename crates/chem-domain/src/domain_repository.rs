// repository.rs
use crate::{DomainError, Molecule, MoleculeFamily};
use serde_json::Value;
use uuid::Uuid;

pub trait DomainRepository: Send + Sync {
  fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;
  fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;
  fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;
  fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;
  fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;
  fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError>;
  fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError>;
  fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError>;
  fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError>;
  fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError>;
  fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;
  fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;
  fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError>;
  fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedMolecularProperty {
  pub id: Uuid,
  pub molecule_inchikey: String,
  pub property_type: String,
  pub value: Value,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  pub metadata: Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedFamilyProperty {
  pub id: Uuid,
  pub family_id: Uuid,
  pub property_type: String,
  pub value: Value,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  pub metadata: Value,
}
