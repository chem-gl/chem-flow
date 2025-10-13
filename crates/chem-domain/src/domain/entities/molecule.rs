//! Molecule Entity
//!
//! Core aggregate root representing a chemical molecule with proper
//! value objects and domain behavior.
use crate::domain::value_objects::{InChI, InChIKey, MolecularFormula, Smiles};
use crate::ports::ProviderMolecule;
use crate::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
/// Molecule entity - aggregate root for molecular information
///
/// Represents a chemical molecule with validated structural identifiers
/// and metadata. Enforces invariants and provides domain behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Molecule {
  id: Uuid,
  inchikey: InChIKey,
  smiles: Smiles,
  inchi: InChI,
  molecular_formula: Option<MolecularFormula>,
  metadata: serde_json::Value,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
  version: u64,
}
impl Molecule {
  /// Create a new molecule with required structural identifiers
  pub fn new(inchikey: InChIKey, smiles: Smiles, inchi: InChI, metadata: serde_json::Value) -> Result<Self, DomainError> {
    let now = Utc::now();
    // Extract molecular formula from InChI if possible
    let molecular_formula =
      if let Some(formula_str) = inchi.molecular_formula() { MolecularFormula::new(formula_str).ok() } else { None };
    let molecule = Self { id: Uuid::new_v4(),
                          inchikey,
                          smiles,
                          inchi,
                          molecular_formula,
                          metadata,
                          created_at: now,
                          updated_at: now,
                          version: 1 };
    molecule.validate()?;
    Ok(molecule)
  }
  /// Create a molecule from parts with full control
  #[allow(clippy::too_many_arguments)]
  pub fn from_parts(id: Uuid,
                    inchikey: InChIKey,
                    smiles: Smiles,
                    inchi: InChI,
                    molecular_formula: Option<MolecularFormula>,
                    metadata: serde_json::Value,
                    created_at: DateTime<Utc>,
                    updated_at: DateTime<Utc>,
                    version: u64)
                    -> Result<Self, DomainError> {
    Ok(Self { id, inchikey, smiles, inchi, molecular_formula, metadata, created_at, updated_at, version })
  }

  /// Create a molecule with simplified arguments (for backward compatibility
  /// and tests)
  pub fn from_simple_parts(inchikey: &str,
                           smiles: &str,
                           inchi: &str,
                           metadata: serde_json::Value)
                           -> Result<Self, DomainError> {
    let inchikey = InChIKey::try_from(inchikey)?;
    let smiles = Smiles::try_from(smiles)?;
    let inchi = InChI::try_from(inchi)?;
    let now = Utc::now();

    Ok(Self { id: Uuid::new_v4(),
              inchikey,
              smiles,
              inchi,
              molecular_formula: None,
              metadata,
              created_at: now,
              updated_at: now,
              version: 1 })
  }

  /// Create a molecule from a provider molecule (used in workflows)
  pub fn from_provider_molecule(provider_mol: ProviderMolecule) -> Result<Self, DomainError> {
    let inchikey = InChIKey::try_from(provider_mol.inchikey.as_str())?;
    let smiles = Smiles::try_from(provider_mol.smiles.as_str())?;
    let inchi = InChI::try_from(provider_mol.inchi.as_str())?;
    let molecular_formula = MolecularFormula::try_from(provider_mol.mol_formula.as_str()).ok();
    let now = Utc::now();

    let molecule = Self { id: Uuid::new_v4(),
                          inchikey,
                          smiles,
                          inchi,
                          molecular_formula,
                          metadata: serde_json::json!({
                            "num_atoms": provider_mol.num_atoms,
                            "mol_weight": provider_mol.mol_weight
                          }),
                          created_at: now,
                          updated_at: now,
                          version: 1 };
    molecule.validate()?;
    Ok(molecule)
  }
  /// Update metadata (creates new version)
  pub fn with_metadata(mut self, metadata: serde_json::Value) -> Result<Self, DomainError> {
    self.metadata = metadata;
    self.updated_at = Utc::now();
    self.version += 1;
    self.validate()?;
    Ok(self)
  }
  /// Update molecular formula (creates new version)
  pub fn with_molecular_formula(mut self, formula: MolecularFormula) -> Result<Self, DomainError> {
    self.molecular_formula = Some(formula);
    self.updated_at = Utc::now();
    self.version += 1;
    self.validate()?;
    Ok(self)
  }
  /// Check if this molecule has aromatic characteristics
  pub fn is_aromatic(&self) -> bool {
    self.smiles.is_aromatic()
  }
  /// Check if this molecule has ring structures
  pub fn has_rings(&self) -> bool {
    self.smiles.has_rings()
  }
  /// Get estimated molecular weight
  pub fn estimated_molecular_weight(&self) -> Option<f64> {
    self.molecular_formula.as_ref().map(|f| f.molecular_weight_estimate())
  }
  /// Get atom count estimate
  pub fn atom_count_estimate(&self) -> usize {
    if let Some(formula) = &self.molecular_formula {
      formula.total_atoms() as usize
    } else {
      self.smiles.atom_count_estimate()
    }
  }
  /// Check structural consistency between identifiers
  pub fn is_structurally_consistent(&self) -> bool {
    // Check if InChI molecular formula matches our formula
    if let (Some(inchi_formula), Some(our_formula)) = (self.inchi.molecular_formula(), &self.molecular_formula) {
      inchi_formula == our_formula.as_str()
    } else {
      true // Can't verify without both formulas
    }
  }
  /// Validate internal consistency
  fn validate(&self) -> Result<(), DomainError> {
    // Check structural consistency
    if !self.is_structurally_consistent() {
      return Err(DomainError::validation("Molecule",
                                         "Inconsistent molecular formula between InChI and stored formula".to_string()));
    }
    // Validate metadata is a proper object
    if !self.metadata.is_object() {
      return Err(DomainError::validation("Molecule", "Metadata must be a JSON object".to_string()));
    }
    // Validate timestamps
    if self.updated_at < self.created_at {
      return Err(DomainError::validation("Molecule", "Updated timestamp cannot be before created timestamp".to_string()));
    }
    // Validate version
    if self.version == 0 {
      return Err(DomainError::validation("Molecule", "Version must be greater than 0".to_string()));
    }
    Ok(())
  }
  // === Getters ===
  pub fn id(&self) -> Uuid {
    self.id
  }
  pub fn inchikey(&self) -> &InChIKey {
    &self.inchikey
  }
  pub fn smiles(&self) -> &Smiles {
    &self.smiles
  }
  pub fn inchi(&self) -> &InChI {
    &self.inchi
  }
  pub fn molecular_formula(&self) -> Option<&MolecularFormula> {
    self.molecular_formula.as_ref()
  }
  pub fn metadata(&self) -> &serde_json::Value {
    &self.metadata
  }
  pub fn created_at(&self) -> DateTime<Utc> {
    self.created_at
  }
  pub fn updated_at(&self) -> DateTime<Utc> {
    self.updated_at
  }
  pub fn version(&self) -> u64 {
    self.version
  }
}
impl fmt::Display for Molecule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Molecule[{}] SMILES: {}, InChIKey: {}", self.id, self.smiles, self.inchikey)
  }
}
/// Builder pattern for creating molecules
pub struct MoleculeBuilder {
  inchikey: Option<InChIKey>,
  smiles: Option<Smiles>,
  inchi: Option<InChI>,
  metadata: serde_json::Value,
}
impl MoleculeBuilder {
  pub fn new() -> Self {
    Self { inchikey: None, smiles: None, inchi: None, metadata: serde_json::json!({}) }
  }
  pub fn inchikey(mut self, inchikey: InChIKey) -> Self {
    self.inchikey = Some(inchikey);
    self
  }
  pub fn smiles(mut self, smiles: Smiles) -> Self {
    self.smiles = Some(smiles);
    self
  }
  pub fn inchi(mut self, inchi: InChI) -> Self {
    self.inchi = Some(inchi);
    self
  }
  pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
    self.metadata = metadata;
    self
  }
  pub fn build(self) -> Result<Molecule, DomainError> {
    let inchikey =
      self.inchikey.ok_or_else(|| DomainError::validation("MoleculeBuilder", "InChIKey is required".to_string()))?;
    let smiles = self.smiles.ok_or_else(|| DomainError::validation("MoleculeBuilder", "SMILES is required".to_string()))?;
    let inchi = self.inchi.ok_or_else(|| DomainError::validation("MoleculeBuilder", "InChI is required".to_string()))?;
    Molecule::new(inchikey, smiles, inchi, self.metadata)
  }
}
impl Default for MoleculeBuilder {
  fn default() -> Self {
    Self::new()
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::value_objects::*;
  fn ethanol_value_objects() -> Result<(InChIKey, Smiles, InChI), DomainError> {
    let inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")?;
    let smiles = Smiles::new("CCO")?;
    let inchi = InChI::new("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3")?;
    Ok((inchikey, smiles, inchi))
  }
  #[test]
  fn molecule_creation_with_valid_data() -> Result<(), DomainError> {
    let (inchikey, smiles, inchi) = ethanol_value_objects()?;
    let metadata = serde_json::json!({"name": "ethanol"});
    let molecule = Molecule::new(inchikey.clone(), smiles.clone(), inchi.clone(), metadata)?;
    assert_eq!(molecule.inchikey(), &inchikey);
    assert_eq!(molecule.smiles(), &smiles);
    assert_eq!(molecule.inchi(), &inchi);
    assert_eq!(molecule.version(), 1);
    assert!(molecule.is_structurally_consistent());
    Ok(())
  }
  #[test]
  fn molecule_builder_pattern() -> Result<(), DomainError> {
    let (inchikey, smiles, inchi) = ethanol_value_objects()?;
    let molecule = MoleculeBuilder::new().inchikey(inchikey)
                                         .smiles(smiles)
                                         .inchi(inchi)
                                         .metadata(serde_json::json!({"source": "test"}))
                                         .build()?;
    assert_eq!(molecule.metadata()["source"], "test");
    Ok(())
  }
  #[test]
  fn molecule_with_metadata_update() -> Result<(), DomainError> {
    let (inchikey, smiles, inchi) = ethanol_value_objects()?;
    let molecule = Molecule::new(inchikey, smiles, inchi, serde_json::json!({}))?;
    let updated = molecule.with_metadata(serde_json::json!({"updated": true}))?;
    assert_eq!(updated.version(), 2);
    assert_eq!(updated.metadata()["updated"], true);
    assert!(updated.updated_at() > updated.created_at());
    Ok(())
  }
  #[test]
  fn molecule_aromatic_detection() -> Result<(), DomainError> {
    let inchikey = InChIKey::new("UHOVQNZJYSORNB-UHFFFAOYSA-N")?; // benzene
    let smiles = Smiles::new("c1ccccc1")?;
    let inchi = InChI::new("InChI=1S/C6H6/c1-2-4-6-5-3-1/h1-6H")?;
    let benzene = Molecule::new(inchikey, smiles, inchi, serde_json::json!({}))?;
    assert!(benzene.is_aromatic());
    assert!(benzene.has_rings());
    Ok(())
  }
  #[test]
  fn builder_missing_required_field_fails() {
    let result = MoleculeBuilder::new().smiles(Smiles::new("CCO").unwrap()).build();
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn invalid_metadata_rejected() -> Result<(), DomainError> {
    let (inchikey, smiles, inchi) = ethanol_value_objects()?;
    // Non-object metadata should be rejected
    let result = Molecule::new(inchikey, smiles, inchi, serde_json::json!("not an object"));
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
    Ok(())
  }
}
