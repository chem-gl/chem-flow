use crate::ports::{FamilyRepository, MoleculeReader, MoleculeWriter};
use crate::{DomainError, Molecule};
use serde_json::Value;
/// Service for molecule-related business operations
///
/// Provides pure business logic for molecule management, validation,
/// and operations while maintaining domain isolation.
pub struct MoleculeService<R>
  where R: MoleculeReader + MoleculeWriter + FamilyRepository
{
  repository: R,
}
impl<R> MoleculeService<R> where R: MoleculeReader + MoleculeWriter + FamilyRepository
{
  /// Create a new molecule service with the given repository
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  /// Create a new molecule with validation
  ///
  /// Validates the molecule structure and saves it to the repository.
  /// Returns the InChIKey of the created molecule.
  pub fn create_molecule(&self, inchikey: &str, smiles: &str, inchi: &str, metadata: Value) -> Result<String, DomainError> {
    let molecule = Molecule::from_simple_parts(inchikey, smiles, inchi, metadata)?;
    // Validate molecule structure
    self.validate_molecule_structure(&molecule)?;
    self.repository.save_molecule(molecule)
  }
  /// Get a molecule by its InChIKey
  pub fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError> {
    self.repository.get_molecule(inchikey)
  }
  /// Validate if a molecule can be deleted (business rule validation)
  ///
  /// Checks if molecule belongs to any families before allowing deletion
  pub fn can_delete_molecule(&self, inchikey: &str) -> Result<bool, DomainError> {
    let molecule = match self.repository.get_molecule(inchikey)? {
      Some(mol) => mol,
      None => return Ok(false), // Doesn't exist, so can't delete
    };
    // Check if molecule belongs to any families
    let families = self.repository.list_families()?;
    for family in families {
      if family.contains(molecule.inchikey()) {
        return Ok(false); // Cannot delete if belongs to family
      }
    }
    Ok(true)
  }
  /// Delete a molecule with business logic validation
  ///
  /// Validates that the molecule can be safely deleted before removing it
  pub fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
    if !self.can_delete_molecule(inchikey)? {
      return Err(DomainError::validation("Molecule", format!("Cannot delete molecule {}: belongs to a family", inchikey)));
    }
    self.repository.delete_molecule(inchikey)
  }
  // Uuid-based deletion method removed; we operate on inchikey IDs consistently.
  /// Find molecules by SMILES pattern (if supported by repository)
  pub fn find_molecules_by_smiles(&self, smiles: &str) -> Result<Vec<Molecule>, DomainError> {
    let all_molecules = self.repository.list_molecules()?;
    Ok(all_molecules.into_iter().filter(|mol| mol.smiles() == smiles).collect())
  }
  /// Validate molecule structural consistency
  ///
  /// Ensures InChI and SMILES are consistent (when validation is available).
  pub fn validate_molecule_structure(&self, molecule: &Molecule) -> Result<bool, DomainError> {
    // In Phase 2, we perform basic validations
    // Phase 4 will add chemical validation via PropertyProvider
    // Basic format validations
    if molecule.smiles().is_empty() {
      return Err(DomainError::validation("molecule", "SMILES cannot be empty"));
    }
    if molecule.inchi().is_empty() {
      return Err(DomainError::validation("molecule", "InChI cannot be empty"));
    }
    if molecule.inchikey().len() != 27 {
      return Err(DomainError::validation("molecule", "Invalid InChIKey length"));
    } // Structure consistency check would be done in Phase 4 with PropertyProvider
    Ok(true)
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{InMemoryDomainRepository, MoleculeFamily};
  use serde_json::json;
  #[test]
  fn test_create_and_retrieve_molecule() {
    let repo = InMemoryDomainRepository::new();
    let service = MoleculeService::new(repo);
    let inchikey = service.create_molecule("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                           "CCO",
                                           "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                           json!({"test": true}))
                          .unwrap();
    let molecule = service.get_molecule(&inchikey).unwrap().unwrap();
    assert_eq!(molecule.smiles(), "CCO");
    assert_eq!(molecule.inchikey(), "LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
  }
  #[test]
  fn test_cannot_delete_molecule_in_family() {
    let repo = InMemoryDomainRepository::new();
    let service = MoleculeService::new(repo);
    // Create molecule
    let molecule = Molecule::from_simple_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                               "CCO",
                                               "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                               json!({})).unwrap();
    let mol_inchikey = MoleculeWriter::save_molecule(&service.repository, molecule.clone()).unwrap();
    // Create family with molecule
    let family = MoleculeFamily::new(vec![molecule], json!({})).unwrap();
    FamilyRepository::save_family(&service.repository, family).unwrap();
    // Should not be able to delete
    assert!(!service.can_delete_molecule(&mol_inchikey).unwrap());
    let result = service.delete_molecule(&mol_inchikey);
    assert!(result.is_err());
  }
  #[test]
  fn test_validate_molecule_structure() {
    let repo = InMemoryDomainRepository::new();
    let service = MoleculeService::new(repo);
    let molecule = Molecule::from_simple_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                               "CCO",
                                               "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                               json!({})).unwrap();
    assert!(service.validate_molecule_structure(&molecule).unwrap());
  }
}
