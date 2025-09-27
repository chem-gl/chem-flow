// family_property.rs
use crate::{DomainError, MoleculeFamily};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct FamilyProperty<'a, V, M> {
  id: Uuid,
  family: &'a MoleculeFamily,
  property_type: String,
  value: V,
  quality: Option<String>,
  preferred: bool,
  value_hash: String,
  metadata: M,
}

impl<'a, V, M> FamilyProperty<'a, V, M>
  where V: Serialize + Clone,
        M: Serialize + Clone
{
  pub fn new(family: &'a MoleculeFamily,
             property_type: &str,
             value: V,
             quality: Option<String>,
             preferred: bool,
             metadata: M)
             -> Result<Self, DomainError> {
    if property_type.trim().is_empty() {
      return Err(DomainError::ValidationError("El tipo de propiedad no puede estar vacío".to_string()));
    }
    let mut hasher = Sha256::new();
    hasher.update(family.family_hash().as_bytes());
    hasher.update(property_type.as_bytes());
    let value_json = serde_json::to_string(&value).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(value_json.as_bytes());
    let metadata_json = serde_json::to_string(&metadata).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(metadata_json.as_bytes());
    let value_hash = format!("{:x}", hasher.finalize());
    Ok(Self { id: Uuid::new_v4(),
              family,
              property_type: property_type.to_string(),
              value,
              quality,
              preferred,
              value_hash,
              metadata })
  }

  pub fn id(&self) -> Uuid {
    self.id
  }

  pub fn family_id(&self) -> Uuid {
    self.family.id()
  }

  pub fn quick_new(family: &'a MoleculeFamily, property_type: &str, value: V) -> Result<Self, DomainError>
    where M: Default
  {
    Self::new(family, property_type, value, None, false, M::default())
  }

  pub fn family(&self) -> &MoleculeFamily {
    self.family
  }

  pub fn property_type(&self) -> &str {
    &self.property_type
  }

  pub fn value(&self) -> &V {
    &self.value
  }

  pub fn quality(&self) -> Option<&str> {
    self.quality.as_deref()
  }

  pub fn preferred(&self) -> bool {
    self.preferred
  }

  pub fn metadata(&self) -> &M {
    &self.metadata
  }

  pub fn value_hash(&self) -> &str {
    &self.value_hash
  }

  pub fn with_quality(&self, quality: Option<String>) -> Result<Self, DomainError> {
    Self::new(self.family,
              &self.property_type,
              self.value.clone(),
              quality,
              self.preferred,
              self.metadata.clone())
  }

  pub fn with_metadata(&self, metadata: M) -> Result<Self, DomainError> {
    Self::new(self.family,
              &self.property_type,
              self.value.clone(),
              self.quality.clone(),
              self.preferred,
              metadata)
  }

  pub fn with_preferred(&self, preferred: bool) -> Result<Self, DomainError> {
    Self::new(self.family,
              &self.property_type,
              self.value.clone(),
              self.quality.clone(),
              preferred,
              self.metadata.clone())
  }

  pub fn is_equivalent(&self, other: &Self) -> bool {
    self.value_hash == other.value_hash
  }

  pub fn verify_integrity(&self) -> Result<bool, DomainError> {
    let mut hasher = Sha256::new();
    hasher.update(self.family.family_hash().as_bytes());
    hasher.update(self.property_type.as_bytes());
    let value_json = serde_json::to_string(&self.value).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(value_json.as_bytes());
    let metadata_json = serde_json::to_string(&self.metadata).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(metadata_json.as_bytes());
    let calculated_hash = format!("{:x}", hasher.finalize());
    Ok(calculated_hash == self.value_hash)
  }
}

impl<'a, V, M> fmt::Display for FamilyProperty<'a, V, M>
  where V: fmt::Debug,
        M: fmt::Debug
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
           "FamilyProperty(id: {}, type: {}, preferred: {})",
           self.id, self.property_type, self.preferred)
  }
}

impl<'a, V, M> PartialEq for FamilyProperty<'a, V, M>
  where V: Serialize + Clone,
        M: Serialize + Clone
{
  fn eq(&self, other: &Self) -> bool {
    self.is_equivalent(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::MoleculeFamily;
  use serde_json::json;

  #[test]
  fn test_family_property_creation() -> Result<(), DomainError> {
    let mol1 = crate::Molecule::from_smiles("CCO")?;
    let mol2 = crate::Molecule::from_smiles("CCN")?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol1, mol2], provenance)?;
    let metadata = json!({"calculation_method": "test"});
    let property = FamilyProperty::new(&family, "average_logP", 2.5f64, Some("high".to_string()), true, metadata)?;
    assert_eq!(property.property_type(), "average_logP");
    assert_eq!(property.value(), &2.5);
    assert!(property.verify_integrity()?);
    Ok(())
  }

  #[test]
  fn test_family_property_equivalence() -> Result<(), DomainError> {
    let mol1 = crate::Molecule::from_smiles("CCO")?;
    let mol2 = crate::Molecule::from_smiles("CCN")?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol1, mol2], provenance)?;
    let metadata = json!({"calculation_method": "test"});
    let prop1 = FamilyProperty::new(&family, "average_logP", 2.5, Some("high".to_string()), true, metadata.clone())?;
    let prop2 = FamilyProperty::new(&family, "average_logP", 2.5, Some("high".to_string()), true, metadata)?;
    assert_eq!(prop1, prop2);
    Ok(())
  }

  #[test]
  fn test_family_property_empty_type() -> Result<(), DomainError> {
    let mol1 = crate::Molecule::from_smiles("CCO")?;
    let mol2 = crate::Molecule::from_smiles("CCN")?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol1, mol2], provenance)?;
    let metadata = json!({"calculation_method": "test"});
    let result = FamilyProperty::new(&family, "", 2.5, Some("high".to_string()), true, metadata);
    assert!(result.is_err());
    Ok(())
  }
}
