//! Entidad MolecularProperty - Propiedad asociada a una molécula con hash para
//! integridad

use crate::{DomainError, Molecule};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::Uuid;

/// Entidad MolecularProperty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MolecularProperty<V, M> {
  id: Uuid,
  molecule: Molecule,
  property_type: String,
  value: V,
  quality: Option<String>,
  preferred: bool,
  value_hash: String,
  metadata: M,
}

impl<V, M> MolecularProperty<V, M>
  where V: Serialize + Clone,
        M: Serialize + Clone
{
  /// Crea una nueva propiedad molecular
  pub fn new(molecule: Molecule,
             property_type: &str,
             value: V,
             quality: Option<String>,
             preferred: bool,
             metadata: M)
             -> Result<Self, DomainError> {
    let normalized_type = property_type.trim().to_lowercase();
    if normalized_type.is_empty() {
      return Err(DomainError::validation("MolecularProperty", "El tipo de propiedad no puede estar vacío"));
    }

    let value_hash = Self::calculate_value_hash(molecule.inchikey(), &normalized_type, &value, &metadata)?;

    Ok(Self { id: Uuid::new_v4(),
              molecule,
              property_type: normalized_type,
              value,
              quality,
              preferred,
              value_hash,
              metadata })
  }

  /// Calcula el hash de la propiedad
  fn calculate_value_hash(inchikey: &str, property_type: &str, value: &V, metadata: &M) -> Result<String, DomainError> {
    let mut hasher = Sha256::new();
    hasher.update(inchikey.as_bytes());
    hasher.update(property_type.as_bytes());
    let value_json = serde_json::to_string(value)?;
    hasher.update(value_json.as_bytes());
    let metadata_json = serde_json::to_string(metadata)?;
    hasher.update(metadata_json.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
  }

  /// Verifica la integridad del hash de la propiedad
  pub fn verify_integrity(&self) -> Result<bool, DomainError> {
    let calculated_hash =
      Self::calculate_value_hash(self.molecule.inchikey(), &self.property_type, &self.value, &self.metadata)?;
    Ok(calculated_hash == self.value_hash)
  }

  // === Getters ===

  pub fn id(&self) -> Uuid {
    self.id
  }

  pub fn molecule(&self) -> &Molecule {
    &self.molecule
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

  pub fn value_hash(&self) -> &str {
    &self.value_hash
  }

  pub fn metadata(&self) -> &M {
    &self.metadata
  }

  // === Métodos de modificación (crean nuevas instancias) ===

  /// Crea una nueva propiedad con calidad actualizada
  pub fn with_quality(&self, quality: Option<String>) -> Result<Self, DomainError> {
    Self::new(self.molecule.clone(),
              &self.property_type,
              self.value.clone(),
              quality,
              self.preferred,
              self.metadata.clone())
  }

  /// Crea una nueva propiedad con metadata actualizado
  pub fn with_metadata(&self, metadata: M) -> Result<Self, DomainError> {
    Self::new(self.molecule.clone(),
              &self.property_type,
              self.value.clone(),
              self.quality.clone(),
              self.preferred,
              metadata)
  }

  /// Crea una nueva propiedad con preferencia actualizada
  pub fn with_preferred(&self, preferred: bool) -> Result<Self, DomainError> {
    Self::new(self.molecule.clone(),
              &self.property_type,
              self.value.clone(),
              self.quality.clone(),
              preferred,
              self.metadata.clone())
  }

  // === Métodos de dominio ===

  /// Verifica si dos propiedades son equivalentes (mismo hash)
  pub fn is_equivalent(&self, other: &Self) -> bool {
    self.value_hash == other.value_hash
  }
}

impl<V, M> fmt::Display for MolecularProperty<V, M>
  where V: fmt::Debug,
        M: fmt::Debug
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
           "MolecularProperty(id: {}, type: {}, preferred: {})",
           self.id, self.property_type, self.preferred)
  }
}

impl<V, M> PartialEq for MolecularProperty<V, M>
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
  use crate::Molecule;
  use serde_json::json;

  #[test]
  fn test_molecular_property_creation() -> Result<(), DomainError> {
    let molecule = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                        "CCO",
                                        "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                        json!({}))?;
    let prop = MolecularProperty::new(molecule,
                                      "molecular_weight",
                                      46.07,
                                      Some("high".to_string()),
                                      true,
                                      json!({"source": "calculated"}))?;
    assert!(prop.verify_integrity()?);
    Ok(())
  }

  #[test]
  fn test_invalid_property_type() {
    let molecule = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                        "CCO",
                                        "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                        json!({})).unwrap();
    let result = MolecularProperty::new(molecule, "   ", 46.07, None, false, json!({}));
    assert!(result.is_err());
  }
}
