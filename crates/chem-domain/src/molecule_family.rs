// molecule_family.rs
use crate::{DomainError, Molecule};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeFamily {
  id: Uuid,
  name: Option<String>,
  description: Option<String>,
  family_hash: String,
  provenance: serde_json::Value,
  frozen: bool,
  molecules: Vec<Molecule>,
}

impl MoleculeFamily {
  pub fn new<I>(molecules: I, provenance: serde_json::Value) -> Result<Self, DomainError>
    where I: IntoIterator<Item = Molecule>
  {
    let mut molecules: Vec<Molecule> = molecules.into_iter().collect();
    if molecules.is_empty() {
      return Err(DomainError::ValidationError("Una familia molecular no puede estar vacía".to_string()));
    }
    let mut seen = HashSet::new();
    molecules.retain(|m| seen.insert(m.inchikey().to_string()));
    let family_hash = Self::calculate_family_hash(&molecules);
    Ok(Self { id: Uuid::new_v4(), name: None, description: None, family_hash, provenance, frozen: true, molecules })
  }

  fn calculate_family_hash(molecules: &[Molecule]) -> String {
    let mut inchikeys: Vec<&str> = molecules.iter().map(|m| m.inchikey()).collect();
    inchikeys.sort();
    let mut hasher = Sha256::new();
    for ik in inchikeys {
      hasher.update(ik.as_bytes());
    }
    format!("{:x}", hasher.finalize())
  }

  pub fn with_name(&self, name: impl Into<String>) -> Self {
    let mut new_family = self.clone();
    new_family.name = Some(name.into());
    new_family.id = Uuid::new_v4();
    new_family
  }

  pub fn with_description(&self, description: impl Into<String>) -> Self {
    let mut new_family = self.clone();
    new_family.description = Some(description.into());
    new_family.id = Uuid::new_v4();
    new_family
  }

  pub fn add_molecule(&self, molecule: Molecule) -> Result<Self, DomainError> {
    if self.molecules.iter().any(|m| m.inchikey() == molecule.inchikey()) {
      return Err(DomainError::ValidationError(format!("Molécula ya existe en la familia: {}", molecule.inchikey())));
    }
    let mut new_molecules = self.molecules.clone();
    new_molecules.push(molecule);
    let family_hash = Self::calculate_family_hash(&new_molecules);
    Ok(Self { id: Uuid::new_v4(),
              name: self.name.clone(),
              description: self.description.clone(),
              family_hash,
              provenance: self.provenance.clone(),
              frozen: true,
              molecules: new_molecules })
  }

  pub fn remove_molecule(&self, inchikey: &str) -> Result<Self, DomainError> {
    let new_molecules: Vec<Molecule> = self.molecules.iter().filter(|m| m.inchikey() != inchikey).cloned().collect();
    if new_molecules.is_empty() {
      return Err(DomainError::ValidationError("No se puede eliminar la última molécula de la familia".to_string()));
    }
    let family_hash = Self::calculate_family_hash(&new_molecules);
    Ok(Self { id: Uuid::new_v4(),
              name: self.name.clone(),
              description: self.description.clone(),
              family_hash,
              provenance: self.provenance.clone(),
              frozen: true,
              molecules: new_molecules })
  }

  pub fn verify_integrity(&self) -> bool {
    Self::calculate_family_hash(&self.molecules) == self.family_hash
  }

  pub fn molecules(&self) -> &[Molecule] {
    &self.molecules
  }

  pub fn len(&self) -> usize {
    self.molecules.len()
  }

  pub fn is_empty(&self) -> bool {
    self.molecules.is_empty()
  }

  pub fn contains(&self, inchikey: &str) -> bool {
    self.molecules.iter().any(|m| m.inchikey() == inchikey)
  }

  pub fn family_hash(&self) -> &str {
    &self.family_hash
  }

  pub fn is_frozen(&self) -> bool {
    self.frozen
  }

  pub fn id(&self) -> Uuid {
    self.id
  }

  pub fn name(&self) -> Option<&str> {
    self.name.as_deref()
  }

  pub fn description(&self) -> Option<&str> {
    self.description.as_deref()
  }

  pub fn provenance(&self) -> &serde_json::Value {
    &self.provenance
  }

  pub fn with_id(&self, id: Uuid) -> Self {
    let mut new_family = self.clone();
    new_family.id = id;
    new_family
  }

  pub fn is_equivalent(&self, other: &Self) -> bool {
    self.family_hash == other.family_hash
  }
}

impl<'a> IntoIterator for &'a MoleculeFamily {
  type Item = &'a Molecule;
  type IntoIter = std::slice::Iter<'a, Molecule>;

  fn into_iter(self) -> Self::IntoIter {
    self.molecules.iter()
  }
}

impl IntoIterator for MoleculeFamily {
  type Item = Molecule;
  type IntoIter = std::vec::IntoIter<Molecule>;

  fn into_iter(self) -> Self::IntoIter {
    self.molecules.into_iter()
  }
}

impl fmt::Display for MoleculeFamily {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
           "MoleculeFamily(id: {}, name: {}, molecules: {})",
           self.id,
           self.name.as_deref().unwrap_or("sin nombre"),
           self.molecules.len())
  }
}

impl PartialEq for MoleculeFamily {
  fn eq(&self, other: &Self) -> bool {
    self.is_equivalent(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_molecule_family_creation() -> Result<(), DomainError> {
    let mol1 = Molecule::from_smiles("CCO")?;
    let mol2 = Molecule::from_smiles("CCN")?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol1, mol2], provenance)?;
    assert_eq!(family.len(), 2);
    assert!(family.verify_integrity());
    Ok(())
  }

  #[test]
  fn test_molecule_family_duplicates() -> Result<(), DomainError> {
    let mol = Molecule::from_smiles("CCO")?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol.clone(), mol], provenance)?;
    assert_eq!(family.len(), 1);
    Ok(())
  }

  #[test]
  fn test_molecule_family_empty() {
    let provenance = json!({"source": "test"});
    let result = MoleculeFamily::new(Vec::<Molecule>::new(), provenance);
    assert!(result.is_err());
  }

  #[test]
  fn test_canonical_hash() -> Result<(), DomainError> {
    let mol1 = Molecule::from_parts("A", "CCO", "InChI1", json!({}))?;
    let mol2 = Molecule::from_parts("B", "CCN", "InChI2", json!({}))?;
    let family1 = MoleculeFamily::new(vec![mol1.clone(), mol2.clone()], json!({}))?;
    let family2 = MoleculeFamily::new(vec![mol2, mol1], json!({}))?;
    assert_eq!(family1.family_hash, family2.family_hash);
    Ok(())
  }
}
