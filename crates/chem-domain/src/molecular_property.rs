// molecular_property.rs
use crate::{DomainError, Molecule};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MolecularProperty<'a, V, M> {
  id: Uuid,
  molecule: &'a Molecule,
  property_type: String,
  value: V,
  quality: Option<String>,
  preferred: bool,
  value_hash: String,
  metadata: M,
}

impl<'a, V, M> MolecularProperty<'a, V, M>
  where V: Serialize + Clone,
        M: Serialize + Clone
{
  pub fn new(molecule: &'a Molecule,
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
    hasher.update(molecule.inchikey().as_bytes());
    hasher.update(property_type.as_bytes());
    let value_json = serde_json::to_string(&value).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(value_json.as_bytes());
    let metadata_json = serde_json::to_string(&metadata).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(metadata_json.as_bytes());
    let value_hash = format!("{:x}", hasher.finalize());
    Ok(Self { id: Uuid::new_v4(),
              molecule,
              property_type: property_type.to_string(),
              value,
              quality,
              preferred,
              value_hash,
              metadata })
  }

  pub fn value_hash(&self) -> &str {
    &self.value_hash
  }

  pub fn id(&self) -> Uuid {
    self.id
  }

  pub fn molecule(&self) -> &Molecule {
    self.molecule
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

  pub fn with_quality(&self, quality: Option<String>) -> Result<Self, DomainError> {
    Self::new(self.molecule,
              &self.property_type,
              self.value.clone(),
              quality,
              self.preferred,
              self.metadata.clone())
  }

  pub fn with_metadata(&self, metadata: M) -> Result<Self, DomainError> {
    Self::new(self.molecule,
              &self.property_type,
              self.value.clone(),
              self.quality.clone(),
              self.preferred,
              metadata)
  }

  pub fn with_preferred(&self, preferred: bool) -> Result<Self, DomainError> {
    Self::new(self.molecule,
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
    hasher.update(self.molecule.inchikey().as_bytes());
    hasher.update(self.property_type.as_bytes());
    let value_json = serde_json::to_string(&self.value).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(value_json.as_bytes());
    let metadata_json = serde_json::to_string(&self.metadata).map_err(|e| DomainError::SerializationError(e.to_string()))?;
    hasher.update(metadata_json.as_bytes());
    let calculated_hash = format!("{:x}", hasher.finalize());
    Ok(calculated_hash == self.value_hash)
  }
}

impl<'a, V, M> fmt::Display for MolecularProperty<'a, V, M>
  where V: fmt::Debug,
        M: fmt::Debug
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
           "MolecularProperty(id: {}, type: {}, preferred: {})",
           self.id, self.property_type, self.preferred)
  }
}

impl<'a, V, M> PartialEq for MolecularProperty<'a, V, M>
  where V: Serialize + Clone,
        M: Serialize + Clone
{
  fn eq(&self, other: &Self) -> bool {
    self.is_equivalent(other)
  }
}
