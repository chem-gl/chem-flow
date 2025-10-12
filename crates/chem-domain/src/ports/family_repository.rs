// ports/family_repository.rs
//! Port para operaciones de familias de moléculas

use crate::{DomainError, Molecule, MoleculeFamily};
use uuid::Uuid;

/// Port para gestionar familias de moléculas
pub trait FamilyRepository: Send + Sync {
  /// Guarda una familia, retorna el UUID
  fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;

  /// Obtiene una familia por su UUID
  fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;

  /// Lista todas las familias
  fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;

  /// Elimina una familia por su UUID
  fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;

  /// Agrega una molécula a una familia
  fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError>;

  /// Remueve una molécula de una familia
  fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError>;
}
