// ports/property_provider.rs
//! Port para proveer funcionalidad química (cálculos, conversiones,
//! estructuras). Será implementado por adapters como RDKit, ChemAxon, etc.
use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// Estructura molecular serializable (reemplaza la de chem-providers)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeStructure {
  /// Representación binaria de la estructura (e.g., MOL block, SDF)
  pub data: Vec<u8>,
  /// Formato de la estructura (e.g., "mol", "sdf", "pdb")
  pub format: String,
}
/// Tipos de propiedades calculables
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyType {
  /// Peso molecular
  MolecularWeight,
  /// LogP (coeficiente de partición)
  LogP,
  /// Área de superficie polar topológica
  TPSA,
  /// Número de donadores de enlaces de hidrógeno
  HBondDonors,
  /// Número de aceptores de enlaces de hidrógeno
  HBondAcceptors,
  /// Número de enlaces rotables
  RotatableBonds,
  /// Propiedad personalizada
  Custom(String),
}
impl std::fmt::Display for PropertyType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PropertyType::MolecularWeight => write!(f, "molecular_weight"),
      PropertyType::LogP => write!(f, "logp"),
      PropertyType::TPSA => write!(f, "tpsa"),
      PropertyType::HBondDonors => write!(f, "hbond_donors"),
      PropertyType::HBondAcceptors => write!(f, "hbond_acceptors"),
      PropertyType::RotatableBonds => write!(f, "rotatable_bonds"),
      PropertyType::Custom(name) => write!(f, "{}", name),
    }
  }
}
/// Información básica de una molécula generada por el provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMolecule {
  /// InChIKey (27 caracteres)
  pub inchikey: String,
  /// InChI estándar
  pub inchi: String,
  /// SMILES canónico
  pub smiles: String,
  /// Número de átomos
  pub num_atoms: u32,
  /// Peso molecular
  pub mol_weight: f64,
  /// Fórmula molecular
  pub mol_formula: String,
  /// Estructura 3D/2D opcional
  pub structure: Option<MoleculeStructure>,
}
/// Port para proveedor de funcionalidad química
pub trait PropertyProvider: Send + Sync {
  /// Genera una molécula a partir de SMILES
  fn get_molecule_from_smiles(&self, smiles: &str) -> Result<ProviderMolecule, DomainError>;
  /// Calcula propiedades para un SMILES dado
  fn calculate_properties(&self,
                          smiles: &str,
                          properties: &[PropertyType])
                          -> Result<HashMap<PropertyType, f64>, DomainError>;
  /// Valida si un SMILES es válido
  fn validate_smiles(&self, smiles: &str) -> Result<bool, DomainError>;
  /// Genera estructura 2D/3D
  fn generate_structure(&self, smiles: &str, format: &str) -> Result<MoleculeStructure, DomainError>;
}
