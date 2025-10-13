//! Command DTOs and Handlers
//!
//! Commands represent write operations in the CQRS pattern.
//! They encapsulate all data needed to perform a specific business operation.

use crate::domain::value_objects::{InChI, InChIKey, Smiles};
use crate::DomainError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Command to create a new molecule from SMILES
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateMoleculeFromSmiles {
  /// SMILES string for the molecule
  pub smiles: String,
  /// Optional metadata for the molecule
  pub metadata: serde_json::Value,
  /// Optional source identifier
  pub source: Option<String>,
}

impl CreateMoleculeFromSmiles {
  pub fn new(smiles: impl Into<String>) -> Self {
    Self { smiles: smiles.into(), metadata: serde_json::json!({}), source: None }
  }

  pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
    self.metadata = metadata;
    self
  }

  pub fn with_source(mut self, source: impl Into<String>) -> Self {
    self.source = Some(source.into());
    self
  }

  /// Validate the command
  pub fn validate(&self) -> Result<(), DomainError> {
    // Validate SMILES format
    Smiles::new(&self.smiles)?;

    // Validate metadata is an object
    if !self.metadata.is_object() {
      return Err(DomainError::validation("CreateMoleculeFromSmiles", "Metadata must be a JSON object".to_string()));
    }

    Ok(())
  }
}

/// Command to create a molecule with complete structural data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateMoleculeComplete {
  /// InChIKey identifier
  pub inchikey: String,
  /// SMILES representation
  pub smiles: String,
  /// InChI representation
  pub inchi: String,
  /// Optional metadata
  pub metadata: serde_json::Value,
}

impl CreateMoleculeComplete {
  pub fn new(inchikey: impl Into<String>, smiles: impl Into<String>, inchi: impl Into<String>) -> Self {
    Self { inchikey: inchikey.into(), smiles: smiles.into(), inchi: inchi.into(), metadata: serde_json::json!({}) }
  }

  pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
    self.metadata = metadata;
    self
  }

  /// Validate the command
  pub fn validate(&self) -> Result<(), DomainError> {
    // Validate all structural identifiers
    InChIKey::new(&self.inchikey)?;
    Smiles::new(&self.smiles)?;
    InChI::new(&self.inchi)?;

    // Validate metadata is an object
    if !self.metadata.is_object() {
      return Err(DomainError::validation("CreateMoleculeComplete", "Metadata must be a JSON object".to_string()));
    }

    Ok(())
  }
}

/// Command to update molecule metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateMoleculeMetadata {
  /// Molecule ID to update
  pub molecule_id: Uuid,
  /// New metadata
  pub metadata: serde_json::Value,
  /// Expected version for optimistic locking
  pub expected_version: Option<u64>,
}

impl UpdateMoleculeMetadata {
  pub fn new(molecule_id: Uuid, metadata: serde_json::Value) -> Self {
    Self { molecule_id, metadata, expected_version: None }
  }

  pub fn with_version(mut self, version: u64) -> Self {
    self.expected_version = Some(version);
    self
  }

  /// Validate the command
  pub fn validate(&self) -> Result<(), DomainError> {
    if !self.metadata.is_object() {
      return Err(DomainError::validation("UpdateMoleculeMetadata", "Metadata must be a JSON object".to_string()));
    }
    Ok(())
  }
}

/// Command to delete a molecule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteMolecule {
  /// Molecule ID to delete
  pub molecule_id: Uuid,
  /// Reason for deletion
  pub reason: String,
  /// Force deletion even if molecule is referenced
  pub force: bool,
}

impl DeleteMolecule {
  pub fn new(molecule_id: Uuid, reason: impl Into<String>) -> Self {
    Self { molecule_id, reason: reason.into(), force: false }
  }

  pub fn force(mut self) -> Self {
    self.force = true;
    self
  }

  /// Validate the command
  pub fn validate(&self) -> Result<(), DomainError> {
    if self.reason.trim().is_empty() {
      return Err(DomainError::validation("DeleteMolecule", "Reason cannot be empty".to_string()));
    }
    Ok(())
  }
}

/// Command to calculate properties for a molecule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalculateMoleculeProperties {
  /// Molecule ID
  pub molecule_id: Uuid,
  /// Properties to calculate
  pub properties: Vec<String>,
  /// Provider to use for calculation
  pub provider: Option<String>,
  /// Force recalculation even if properties exist
  pub force_recalculation: bool,
}

impl CalculateMoleculeProperties {
  pub fn new(molecule_id: Uuid, properties: Vec<String>) -> Self {
    Self { molecule_id, properties, provider: None, force_recalculation: false }
  }

  pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
    self.provider = Some(provider.into());
    self
  }

  pub fn force_recalculation(mut self) -> Self {
    self.force_recalculation = true;
    self
  }

  /// Validate the command
  pub fn validate(&self) -> Result<(), DomainError> {
    if self.properties.is_empty() {
      return Err(DomainError::validation("CalculateMoleculeProperties",
                                         "At least one property must be specified".to_string()));
    }
    Ok(())
  }
}

/// Command result type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResult<T> {
  /// Success flag
  pub success: bool,
  /// Result data (if successful)
  pub data: Option<T>,
  /// Error message (if failed)
  pub error: Option<String>,
  /// Additional metadata
  pub metadata: serde_json::Value,
}

impl<T> CommandResult<T> {
  pub fn success(data: T) -> Self {
    Self { success: true, data: Some(data), error: None, metadata: serde_json::json!({}) }
  }

  pub fn failure(error: impl Into<String>) -> Self {
    Self { success: false, data: None, error: Some(error.into()), metadata: serde_json::json!({}) }
  }

  pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
    self.metadata = metadata;
    self
  }
}

impl<T> From<Result<T, DomainError>> for CommandResult<T> {
  fn from(result: Result<T, DomainError>) -> Self {
    match result {
      Ok(data) => CommandResult::success(data),
      Err(error) => CommandResult::failure(error.to_string()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_molecule_from_smiles_validation() {
    let valid_command = CreateMoleculeFromSmiles::new("CCO").with_metadata(serde_json::json!({"name": "ethanol"}));

    assert!(valid_command.validate().is_ok());

    let invalid_command = CreateMoleculeFromSmiles { smiles: "".to_string(), // Empty SMILES
                                                     metadata: serde_json::json!({}),
                                                     source: None };

    assert!(invalid_command.validate().is_err());
  }

  #[test]
  fn create_molecule_complete_validation() {
    let valid_command =
      CreateMoleculeComplete::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N", "CCO", "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3");

    assert!(valid_command.validate().is_ok());

    let invalid_command = CreateMoleculeComplete::new("INVALID", // Invalid InChIKey
                                                      "CCO",
                                                      "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3");

    assert!(invalid_command.validate().is_err());
  }

  #[test]
  fn delete_molecule_validation() {
    let valid_command = DeleteMolecule::new(Uuid::new_v4(), "No longer needed");
    assert!(valid_command.validate().is_ok());

    let invalid_command = DeleteMolecule::new(Uuid::new_v4(), "   "); // Empty reason
    assert!(invalid_command.validate().is_err());
  }

  #[test]
  fn command_result_creation() {
    let success: CommandResult<Uuid> = CommandResult::success(Uuid::new_v4());
    assert!(success.success);
    assert!(success.data.is_some());
    assert!(success.error.is_none());

    let failure: CommandResult<Uuid> = CommandResult::failure("Something went wrong");
    assert!(!failure.success);
    assert!(failure.data.is_none());
    assert!(failure.error.is_some());
  }

  #[test]
  fn command_result_from_domain_result() {
    let success_result: Result<Uuid, DomainError> = Ok(Uuid::new_v4());
    let command_result: CommandResult<Uuid> = success_result.into();
    assert!(command_result.success);

    let error_result: Result<Uuid, DomainError> = Err(DomainError::validation("test", "error"));
    let command_result: CommandResult<Uuid> = error_result.into();
    assert!(!command_result.success);
  }
}
