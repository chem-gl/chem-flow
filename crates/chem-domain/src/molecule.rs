// molecule.rs
use crate::DomainError;
use chem_providers::{ChemEngine, ChemEngineInterface};
use chrono::Utc;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(not(any(test, feature = "mock_rdkit")))]
static ENGINE: Lazy<Result<ChemEngine, DomainError>> = Lazy::new(|| {
  ChemEngine::init().map_err(|e| DomainError::ExternalError(format!("Error al inicializar el motor químico: {}", e)))
});

#[cfg(any(test, feature = "mock_rdkit"))]
static ENGINE: Lazy<Result<Box<dyn ChemEngineInterface>, DomainError>> = Lazy::new(|| {
  // Use the mock engine for tests
  let engine = {
    #[cfg(feature = "mock_rdkit")]
    {
      // Create a mock implementation
      let mut mock = chem_providers::MockChemEngineInterface::new();

      // Configure the mock to return predictable responses
      mock.expect_get_molecule().returning(|smiles| {
                                  Ok(chem_providers::Molecule { inchikey: format!("MOCK-{}-ABCDEFGHIJ-P", smiles),
                                                                inchi: format!("InChI=MOCK/{}", smiles),
                                                                smiles: smiles.to_string(),
                                                                num_atoms: (smiles.len() as u32).max(1),
                                                                mol_weight: 12.01 * (smiles.len() as f64).max(1.0),
                                                                mol_formula: format!("C{}", smiles.len().max(1)),
                                                                structure: None })
                                });

      mock
    }

    #[cfg(not(feature = "mock_rdkit"))]
    {
      // Fall back to regular implementation if mock_rdkit isn't enabled
      ChemEngine::init().map_err(|e| DomainError::ExternalError(format!("Error al inicializar el motor químico: {}", e)))?
    }
  };

  Ok(Box::new(engine))
});

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Molecule {
  inchikey: String,
  smiles: String,
  inchi: String,
  metadata: serde_json::Value,
}

impl Molecule {
  fn new(inchikey: &str, smiles: &str, inchi: &str, metadata: serde_json::Value) -> Result<Self, DomainError> {
    let normalized_inchikey = inchikey.to_uppercase();
    if normalized_inchikey.len() != 27 {
      return Err(DomainError::ValidationError(format!("InChIKey debe tener exactamente 27 caracteres, pero tiene {} y \
                                                       es: {} la entrada fue: {} y el smiles: {}",
                                                      normalized_inchikey.len(),
                                                      normalized_inchikey,
                                                      inchikey,
                                                      smiles)));
    }
    if normalized_inchikey.matches('-').count() != 2 {
      return Err(DomainError::ValidationError("InChIKey debe contener exactamente dos guiones".to_string()));
    }
    let parts: Vec<&str> = normalized_inchikey.split('-').collect();
    if parts.len() != 3
       || !parts[0].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
       || !parts[1].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
       || !parts[2].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
      return Err(DomainError::ValidationError("Formato InChIKey inválido o contiene caracteres inválidos".to_string()));
    }
    if smiles.trim().is_empty() {
      return Err(DomainError::ValidationError("SMILES no puede estar vacío".to_string()));
    }
    if inchi.trim().is_empty() {
      return Err(DomainError::ValidationError("InChI no puede estar vacío".to_string()));
    }
    Ok(Self { inchikey: normalized_inchikey, smiles: smiles.to_string(), inchi: inchi.to_string(), metadata })
  }

  pub fn from_parts(inchikey: &str, smiles: &str, inchi: &str, metadata: serde_json::Value) -> Result<Self, DomainError> {
    Self::new(inchikey, smiles, inchi, metadata)
  }

  pub fn from_smiles(smiles: &str) -> Result<Self, DomainError> {
    if smiles.trim().is_empty() {
      return Err(DomainError::ValidationError("SMILES de entrada no puede estar vacío".to_string()));
    }

    // Obtain molecule from the appropriate engine based on feature flags
    let chem_molecule = {
      #[cfg(not(any(test, feature = "mock_rdkit")))]
      {
        let engine: &ChemEngine = ENGINE.as_ref().map_err(|e| e.clone())?;
        ChemEngineInterface::get_molecule(engine, smiles).map_err(|e| {
                                                           DomainError::ExternalError(format!("Error al procesar SMILES: \
                                                                                               {}",
                                                                                              e))
                                                         })?
      }

      #[cfg(any(test, feature = "mock_rdkit"))]
      {
        let engine = ENGINE.as_ref().map_err(|e| e.clone())?;
        engine.get_molecule(smiles).map_err(|e| DomainError::ExternalError(format!("Error al procesar SMILES: {}", e)))?
      }
    };

    // Base metadata
    let mut meta = serde_json::json!({
      "source": "created_from_smiles",
      "original_smiles": smiles,
      "generation_timestamp": Utc::now().to_rfc3339(),
    });
    // If provider returned structure, insert it into metadata so persistence can
    // save it.
    if let Some(structure) = chem_molecule.structure {
      // serialize the structure into a JSON value and attach under key "structure"
      let struct_val = serde_json::to_value(&structure)?;
      meta["structure"] = struct_val;
    }

    Self::new(&chem_molecule.inchikey, &chem_molecule.smiles, &chem_molecule.inchi, meta)
  }

  pub fn smiles(&self) -> &str {
    &self.smiles
  }

  pub fn inchikey(&self) -> &str {
    &self.inchikey
  }

  pub fn inchi(&self) -> &str {
    &self.inchi
  }

  pub fn metadata(&self) -> &serde_json::Value {
    &self.metadata
  }

  pub fn is_same(&self, other: &Molecule) -> bool {
    self.inchikey == other.inchikey
  }
}

impl fmt::Display for Molecule {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
           "Molecule(SMILES: {}, InChI: {}, InChIKey: {})",
           self.smiles, self.inchi, self.inchikey)
  }
}
