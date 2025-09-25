use crate::DomainError;
use crate::{Molecule, MoleculeFamily};
use serde_json::Value as JsonValue;
use uuid::Uuid;
/// DTO para persistir una propiedad de familia de forma independiente
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedFamilyProperty {
  pub id: Uuid,
  pub family_id: Uuid,
  pub property_type: String,
  pub value: JsonValue,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  pub metadata: JsonValue,
}
/// DTO para persistir una propiedad molecular de forma independiente
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedMolecularProperty {
  pub id: Uuid,
  pub molecule_inchikey: String,
  pub property_type: String,
  pub value: JsonValue,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  pub metadata: JsonValue,
}
/// Trait que define operaciones de persistencia para el dominio químico.
pub trait DomainRepository: Send + Sync {
  /// Guarda una familia molecular y devuelve su `Uuid`.
  fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;
  /// Recupera una familia por su `Uuid`.
  fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;
  /// Guarda una molécula y devuelve su InChIKey.
  fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;
  /// Obtiene una molécula por su InChIKey.
  fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;
  /// Lista todas las familias (útil para pruebas).
  fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;
  /// Guarda una propiedad de familia (persistible)
  fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError>;
  /// Recupera propiedades de familia por family_id
  fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError>;
  /// Guarda una propiedad molecular
  fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError>;
  /// Recupera propiedades moleculares por inchikey
  fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError>;
  /// Lista todas las moléculas disponibles (útil para menús y selección).
  fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError>;
  /// Elimina una molécula del repositorio. No permite eliminar si la molécula
  /// forma parte de alguna familia; en ese caso retorna ValidationError.
  fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;
  /// Elimina una familia (y sus propiedades y mapeos) del repositorio.
  fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;
  /// Agrega una molécula a una familia existente y persiste la nueva versión
  /// retornando el nuevo `Uuid` de la familia.
  fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError>;
  /// Remueve una molécula de una familia existente y persiste la nueva
  /// versión retornando el nuevo `Uuid` de la familia.
  fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError>;
}
