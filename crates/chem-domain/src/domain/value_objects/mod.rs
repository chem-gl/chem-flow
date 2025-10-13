//! Value Objects Module
//!
//! This module contains all value objects used throughout the domain.
//! Value objects are immutable, self-validating objects that represent
//! descriptive aspects of the domain with no conceptual identity.
pub mod inchi;
pub mod inchikey;
pub mod molecular_formula;
pub mod smiles;
pub use inchi::InChI;
pub use inchikey::InChIKey;
pub use molecular_formula::MolecularFormula;
pub use smiles::Smiles;
#[cfg(test)]
mod integration_tests {
  use super::*;
  use crate::DomainError;
  #[test]
  fn value_objects_work_together() -> Result<(), DomainError> {
    // Create related value objects for ethanol
    let smiles = Smiles::new("CCO")?;
    let inchi = InChI::new("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3")?;
    let _inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")?;
    let formula = MolecularFormula::new("C2H6O")?;
    // Verify they represent the same molecule conceptually
    assert_eq!(inchi.molecular_formula(), Some("C2H6O"));
    assert_eq!(formula.as_str(), "C2H6O");
    assert_eq!(smiles.atom_count_estimate(), 3); // C, C, O
    assert_eq!(formula.total_atoms(), 9); // 2C + 6H + 1O
                                          // Verify immutability and cloning
    let smiles_clone = smiles.clone();
    assert_eq!(smiles, smiles_clone);
    Ok(())
  }
  #[test]
  fn value_objects_are_serializable() -> Result<(), DomainError> {
    let smiles = Smiles::new("CCO")?;
    let json = serde_json::to_string(&smiles).unwrap();
    let deserialized: Smiles = serde_json::from_str(&json).unwrap();
    assert_eq!(smiles, deserialized);
    Ok(())
  }
}
