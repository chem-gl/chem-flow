//! External Provider Ports
//!
//! Contracts for external chemical information providers and services.
//! These ports abstract away specific implementations (RDKit, ChemAxon, etc.).

use crate::domain::value_objects::{InChI, InChIKey, MolecularFormula, Smiles};
use crate::DomainError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Chemical property types that can be calculated
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyType {
  /// Molecular weight in g/mol
  MolecularWeight,
  /// Octanol-water partition coefficient
  LogP,
  /// Topological polar surface area
  TPSA,
  /// Number of hydrogen bond donors
  HydrogenBondDonors,
  /// Number of hydrogen bond acceptors
  HydrogenBondAcceptors,
  /// Number of rotatable bonds
  RotatableBonds,
  /// Number of rings
  RingCount,
  /// Number of aromatic rings
  AromaticRingCount,
  /// Lipinski rule of five compliance
  LipinskiCompliance,
  /// Custom property by name
  Custom(String),
}

impl std::fmt::Display for PropertyType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PropertyType::MolecularWeight => write!(f, "molecular_weight"),
      PropertyType::LogP => write!(f, "logp"),
      PropertyType::TPSA => write!(f, "tpsa"),
      PropertyType::HydrogenBondDonors => write!(f, "hbond_donors"),
      PropertyType::HydrogenBondAcceptors => write!(f, "hbond_acceptors"),
      PropertyType::RotatableBonds => write!(f, "rotatable_bonds"),
      PropertyType::RingCount => write!(f, "ring_count"),
      PropertyType::AromaticRingCount => write!(f, "aromatic_ring_count"),
      PropertyType::LipinskiCompliance => write!(f, "lipinski_compliance"),
      PropertyType::Custom(name) => write!(f, "{}", name),
    }
  }
}

/// Molecular structure representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeStructure {
  /// Binary structure data (e.g., MOL block, SDF)
  pub data: Vec<u8>,
  /// Format identifier (e.g., "mol", "sdf", "pdb")
  pub format: String,
  /// Optional 3D coordinates flag
  pub has_3d_coords: bool,
}

/// Chemical provider molecule representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMolecule {
  /// InChIKey identifier
  pub inchikey: InChIKey,
  /// InChI string
  pub inchi: InChI,
  /// Canonical SMILES
  pub smiles: Smiles,
  /// Molecular formula
  pub molecular_formula: MolecularFormula,
  /// Number of atoms
  pub atom_count: u32,
  /// Molecular weight
  pub molecular_weight: f64,
  /// Optional 3D structure
  pub structure: Option<MoleculeStructure>,
  /// Provider-specific metadata
  pub metadata: serde_json::Value,
}

/// Property calculation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyValue {
  /// The calculated value
  pub value: f64,
  /// Confidence or quality score (0.0 to 1.0)
  pub confidence: Option<f64>,
  /// Calculation method or algorithm used
  pub method: Option<String>,
  /// Additional metadata about the calculation
  pub metadata: serde_json::Value,
}

/// Chemical structure validation and conversion
#[async_trait]
pub trait ChemicalValidator: Send + Sync {
  /// Validate a SMILES string
  async fn validate_smiles(&self, smiles: &Smiles) -> Result<bool, DomainError>;

  /// Validate an InChI string
  async fn validate_inchi(&self, inchi: &InChI) -> Result<bool, DomainError>;

  /// Check consistency between SMILES and InChI
  async fn check_consistency(&self, smiles: &Smiles, inchi: &InChI) -> Result<bool, DomainError>;
}

/// Chemical structure conversion between formats
#[async_trait]
pub trait ChemicalConverter: Send + Sync {
  /// Generate InChI from SMILES
  async fn smiles_to_inchi(&self, smiles: &Smiles) -> Result<InChI, DomainError>;

  /// Generate InChIKey from InChI
  async fn inchi_to_inchikey(&self, inchi: &InChI) -> Result<InChIKey, DomainError>;

  /// Generate canonical SMILES from any SMILES
  async fn canonicalize_smiles(&self, smiles: &Smiles) -> Result<Smiles, DomainError>;

  /// Extract molecular formula from structure
  async fn extract_molecular_formula(&self, smiles: &Smiles) -> Result<MolecularFormula, DomainError>;
}

/// Property calculation provider
#[async_trait]
pub trait PropertyCalculator: Send + Sync {
  /// Calculate specific properties for a molecule
  async fn calculate_properties(&self,
                                smiles: &Smiles,
                                properties: &[PropertyType])
                                -> Result<HashMap<PropertyType, PropertyValue>, DomainError>;

  /// Calculate a single property
  async fn calculate_property(&self, smiles: &Smiles, property: &PropertyType) -> Result<PropertyValue, DomainError>;

  /// Get list of supported properties
  async fn supported_properties(&self) -> Result<Vec<PropertyType>, DomainError>;

  /// Batch calculate properties for multiple molecules
  async fn calculate_batch(&self,
                           molecules: &[(Smiles, Vec<PropertyType>)])
                           -> Result<Vec<HashMap<PropertyType, PropertyValue>>, DomainError>;
}

/// Structure generation and manipulation
#[async_trait]
pub trait StructureGenerator: Send + Sync {
  /// Generate 2D coordinates for a molecule
  async fn generate_2d_coords(&self, smiles: &Smiles) -> Result<MoleculeStructure, DomainError>;

  /// Generate 3D coordinates for a molecule
  async fn generate_3d_coords(&self, smiles: &Smiles) -> Result<MoleculeStructure, DomainError>;

  /// Optimize molecular geometry
  async fn optimize_geometry(&self, structure: &MoleculeStructure) -> Result<MoleculeStructure, DomainError>;

  /// Generate multiple conformers
  async fn generate_conformers(&self, smiles: &Smiles, max_conformers: usize)
                               -> Result<Vec<MoleculeStructure>, DomainError>;
}

/// Molecular similarity and searching
#[async_trait]
pub trait SimilarityCalculator: Send + Sync {
  /// Calculate Tanimoto similarity between two molecules
  async fn tanimoto_similarity(&self, mol1: &Smiles, mol2: &Smiles) -> Result<f64, DomainError>;

  /// Calculate molecular fingerprint
  async fn calculate_fingerprint(&self, smiles: &Smiles) -> Result<Vec<u8>, DomainError>;

  /// Find similar molecules from a dataset
  async fn find_similar(&self, query: &Smiles, dataset: &[Smiles], threshold: f64)
                        -> Result<Vec<(usize, f64)>, DomainError>;
}

/// Complete chemical provider interface
///
/// Combines all chemical capabilities into a single interface.
/// Implementations can provide all or subset of these capabilities.
pub trait ChemicalProvider:
  ChemicalValidator + ChemicalConverter + PropertyCalculator + StructureGenerator + SimilarityCalculator + Send + Sync
{
  /// Get provider information
  fn provider_info(&self) -> ProviderInfo;
}

/// Information about a chemical provider
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInfo {
  /// Provider name (e.g., "RDKit", "ChemAxon")
  pub name: String,
  /// Provider version
  pub version: String,
  /// Supported capabilities
  pub capabilities: Vec<String>,
  /// License information
  pub license: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;

  // Mock implementation for testing
  struct MockPropertyCalculator;

  #[async_trait]
  impl PropertyCalculator for MockPropertyCalculator {
    async fn calculate_properties(&self,
                                  _smiles: &Smiles,
                                  properties: &[PropertyType])
                                  -> Result<HashMap<PropertyType, PropertyValue>, DomainError> {
      let mut result = HashMap::new();
      for prop in properties {
        result.insert(prop.clone(),
                      PropertyValue { value: 100.0, // Mock value
                                      confidence: Some(0.95),
                                      method: Some("mock".to_string()),
                                      metadata: serde_json::json!({}) });
      }
      Ok(result)
    }

    async fn calculate_property(&self, _smiles: &Smiles, _property: &PropertyType) -> Result<PropertyValue, DomainError> {
      Ok(PropertyValue { value: 100.0,
                         confidence: Some(0.95),
                         method: Some("mock".to_string()),
                         metadata: serde_json::json!({}) })
    }

    async fn supported_properties(&self) -> Result<Vec<PropertyType>, DomainError> {
      Ok(vec![PropertyType::MolecularWeight, PropertyType::LogP, PropertyType::TPSA,])
    }

    async fn calculate_batch(&self,
                             molecules: &[(Smiles, Vec<PropertyType>)])
                             -> Result<Vec<HashMap<PropertyType, PropertyValue>>, DomainError> {
      let mut results = Vec::new();
      for (_, properties) in molecules {
        let mut result = HashMap::new();
        for prop in properties {
          result.insert(prop.clone(),
                        PropertyValue { value: 100.0,
                                        confidence: Some(0.95),
                                        method: Some("mock".to_string()),
                                        metadata: serde_json::json!({}) });
        }
        results.push(result);
      }
      Ok(results)
    }
  }

  #[tokio::test]
  async fn property_calculator_interface() {
    let calculator: Arc<dyn PropertyCalculator> = Arc::new(MockPropertyCalculator);
    let smiles = Smiles::new("CCO").unwrap();

    let properties = vec![PropertyType::MolecularWeight, PropertyType::LogP];
    let result = calculator.calculate_properties(&smiles, &properties).await;

    assert!(result.is_ok());
    let props = result.unwrap();
    assert_eq!(props.len(), 2);
    assert!(props.contains_key(&PropertyType::MolecularWeight));
    assert!(props.contains_key(&PropertyType::LogP));
  }

  #[test]
  fn property_type_display() {
    assert_eq!(PropertyType::MolecularWeight.to_string(), "molecular_weight");
    assert_eq!(PropertyType::Custom("test".to_string()).to_string(), "test");
  }

  #[test]
  fn provider_molecule_creation() -> Result<(), DomainError> {
    let provider_molecule = ProviderMolecule { inchikey: InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")?,
                                               inchi: InChI::new("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3")?,
                                               smiles: Smiles::new("CCO")?,
                                               molecular_formula: MolecularFormula::new("C2H6O")?,
                                               atom_count: 9,
                                               molecular_weight: 46.069,
                                               structure: None,
                                               metadata: serde_json::json!({}) };

    assert_eq!(provider_molecule.atom_count, 9);
    assert_eq!(provider_molecule.molecular_weight, 46.069);
    Ok(())
  }
}
