//! Use Cases para operaciones de dominio
//!
//! Los use cases encapsulan la lógica de aplicación y orquestan las
//! operaciones del dominio a través de los puertos.
use crate::ports::{FamilyRepository, MoleculeReader, MoleculeWriter, PropertyRepository};
use crate::{DomainError, Molecule, MoleculeFamily, OwnedFamilyProperty, OwnedMolecularProperty};
use uuid::Uuid;
// ============================================================================
// MOLECULE USE CASES
// ============================================================================
/// Use case para crear una molécula
pub struct CreateMoleculeUseCase<R: MoleculeWriter> {
  repository: R,
}
impl<R: MoleculeWriter> CreateMoleculeUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, molecule: Molecule) -> Result<String, DomainError> {
    self.repository.save_molecule(molecule)
  }
}
/// Use case para obtener una molécula
pub struct GetMoleculeUseCase<R: MoleculeReader> {
  repository: R,
}
impl<R: MoleculeReader> GetMoleculeUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError> {
    self.repository.get_molecule(inchikey)
  }
}
/// Use case para listar moléculas
pub struct ListMoleculesUseCase<R: MoleculeReader> {
  repository: R,
}
impl<R: MoleculeReader> ListMoleculesUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self) -> Result<Vec<Molecule>, DomainError> {
    self.repository.list_molecules()
  }
}
/// Use case para eliminar una molécula
pub struct DeleteMoleculeUseCase<R: MoleculeWriter> {
  repository: R,
}
impl<R: MoleculeWriter> DeleteMoleculeUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, inchikey: &str) -> Result<(), DomainError> {
    self.repository.delete_molecule(inchikey)
  }
}
// ============================================================================
// FAMILY USE CASES
// ============================================================================
/// Use case para crear una familia
pub struct CreateFamilyUseCase<R: FamilyRepository> {
  repository: R,
}
impl<R: FamilyRepository> CreateFamilyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, family: MoleculeFamily) -> Result<Uuid, DomainError> {
    self.repository.save_family(family)
  }
}
/// Use case para obtener una familia
pub struct GetFamilyUseCase<R: FamilyRepository> {
  repository: R,
}
impl<R: FamilyRepository> GetFamilyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError> {
    self.repository.get_family(id)
  }
}
/// Use case para listar familias
pub struct ListFamiliesUseCase<R: FamilyRepository> {
  repository: R,
}
impl<R: FamilyRepository> ListFamiliesUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self) -> Result<Vec<MoleculeFamily>, DomainError> {
    self.repository.list_families()
  }
}
/// Use case para eliminar una familia
pub struct DeleteFamilyUseCase<R: FamilyRepository> {
  repository: R,
}
impl<R: FamilyRepository> DeleteFamilyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, id: &Uuid) -> Result<(), DomainError> {
    self.repository.delete_family(id)
  }
}
/// Use case para agregar molécula a familia
pub struct AddMoleculeToFamilyUseCase<R: FamilyRepository> {
  repository: R,
}
impl<R: FamilyRepository> AddMoleculeToFamilyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError> {
    self.repository.add_molecule_to_family(family_id, molecule)
  }
}
/// Use case para remover molécula de familia
pub struct RemoveMoleculeFromFamilyUseCase<R: FamilyRepository> {
  repository: R,
}
impl<R: FamilyRepository> RemoveMoleculeFromFamilyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError> {
    self.repository.remove_molecule_from_family(family_id, inchikey)
  }
}
// ============================================================================
// PROPERTY USE CASES
// ============================================================================
/// Use case para guardar propiedad molecular
pub struct SaveMolecularPropertyUseCase<R: PropertyRepository> {
  repository: R,
}
impl<R: PropertyRepository> SaveMolecularPropertyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, property: OwnedMolecularProperty) -> Result<Uuid, DomainError> {
    self.repository.save_molecular_property(property)
  }
}
/// Use case para obtener propiedades moleculares
pub struct GetMolecularPropertiesUseCase<R: PropertyRepository> {
  repository: R,
}
impl<R: PropertyRepository> GetMolecularPropertiesUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError> {
    self.repository.get_molecular_properties(inchikey)
  }
}
/// Use case para guardar propiedad de familia
pub struct SaveFamilyPropertyUseCase<R: PropertyRepository> {
  repository: R,
}
impl<R: PropertyRepository> SaveFamilyPropertyUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, property: OwnedFamilyProperty) -> Result<Uuid, DomainError> {
    self.repository.save_family_property(property)
  }
}
/// Use case para obtener propiedades de familia
pub struct GetFamilyPropertiesUseCase<R: PropertyRepository> {
  repository: R,
}
impl<R: PropertyRepository> GetFamilyPropertiesUseCase<R> {
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  pub fn execute(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError> {
    self.repository.get_family_properties(family_id)
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::InMemoryDomainRepository;
  use serde_json::json;
  #[test]
  fn test_create_molecule_use_case() {
    let repo = InMemoryDomainRepository::new();
    let use_case = CreateMoleculeUseCase::new(repo);
    let molecule = Molecule::from_parts("AAAAAAAAAAAAAA-BBBBBBBBBB-C",
                                        "CCO",
                                        "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                        json!({})).unwrap();
    let result = use_case.execute(molecule);
    assert!(result.is_ok());
  }
  #[test]
  fn test_get_molecule_use_case() {
    let repo = InMemoryDomainRepository::new();
    let create_uc = CreateMoleculeUseCase::new(repo.clone());
    let get_uc = GetMoleculeUseCase::new(repo);
    let molecule = Molecule::from_parts("AAAAAAAAAAAAAA-BBBBBBBBBB-C",
                                        "CCO",
                                        "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                        json!({})).unwrap();
    let inchikey = create_uc.execute(molecule).unwrap();
    let result = get_uc.execute(&inchikey);
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
  }
}
