// ports/property_repository.rs
//! Port para operaciones de propiedades moleculares y de familias
use crate::DomainError;
use serde_json::Value;
use uuid::Uuid;
/// Propiedad molecular serializable (owned)
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
/// Propiedad de familia serializable (owned)
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
/// Port para gestionar propiedades
pub trait PropertyRepository: Send + Sync {
  /// Guarda una propiedad de familia
  fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError>;
  /// Obtiene propiedades de una familia
  fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError>;
  /// Guarda una propiedad molecular
  fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError>;
  /// Obtiene propiedades de una molécula
  fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError>;
}
