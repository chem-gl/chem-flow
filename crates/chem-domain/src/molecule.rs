// molecule.rs
//! Entidad Molecule - Representación inmutable de una molécula

use crate::DomainError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Entidad Molecule (inmutable)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Molecule {
  inchikey: String,
  smiles: String,
  inchi: String,
  metadata: serde_json::Value,
}

impl Molecule {
  /// Constructor interno con validación
  fn new(inchikey: &str, smiles: &str, inchi: &str, metadata: serde_json::Value) -> Result<Self, DomainError> {
    // Validar InChIKey
    let normalized_inchikey = inchikey.to_uppercase();
    if normalized_inchikey.len() != 27 {
      return Err(DomainError::invalid_format("inchikey",
                                             inchikey,
                                             format!("debe tener 27 caracteres, tiene {}", normalized_inchikey.len())));
    }
    if normalized_inchikey.matches('-').count() != 2 {
      return Err(DomainError::invalid_format("inchikey", inchikey, "debe contener exactamente dos guiones"));
    }

    let parts: Vec<&str> = normalized_inchikey.split('-').collect();
    if parts.len() != 3
       || !parts[0].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
       || !parts[1].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
       || !parts[2].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
      return Err(DomainError::invalid_format("inchikey",
                                             inchikey,
                                             "contiene caracteres inválidos (solo A-Z y 0-9 permitidos)"));
    }

    // Validar SMILES
    if smiles.trim().is_empty() {
      return Err(DomainError::validation("Molecule", "SMILES no puede estar vacío"));
    }

    // Validar InChI
    if inchi.trim().is_empty() {
      return Err(DomainError::validation("Molecule", "InChI no puede estar vacío"));
    }

    Ok(Self { inchikey: normalized_inchikey, smiles: smiles.to_string(), inchi: inchi.to_string(), metadata })
  }

  /// Crea una molécula desde sus partes componentes (para persistencia)
  ///
  /// # Validaciones
  /// - InChIKey debe tener 27 caracteres con formato válido
  /// - SMILES no puede estar vacío
  /// - InChI no puede estar vacío
  pub fn from_parts(inchikey: &str, smiles: &str, inchi: &str, metadata: serde_json::Value) -> Result<Self, DomainError> {
    Self::new(inchikey, smiles, inchi, metadata)
  }

  /// Crea una molécula desde SMILES usando un PropertyProvider
  ///
  /// Este método debe ser llamado desde un servicio de dominio que tenga
  /// acceso al PropertyProvider inyectado.
  pub fn from_provider_molecule(original_smiles: &str,
                                provider_molecule: crate::ports::ProviderMolecule)
                                -> Result<Self, DomainError> {
    // Base metadata
    let mut meta = serde_json::json!({
        "source": "created_from_smiles",
        "original_smiles": original_smiles,
        "generation_timestamp": Utc::now().to_rfc3339(),
        "mol_weight": provider_molecule.mol_weight,
        "mol_formula": provider_molecule.mol_formula,
        "num_atoms": provider_molecule.num_atoms,
    });

    // Si hay estructura, agregarla al metadata
    if let Some(structure) = provider_molecule.structure {
      let struct_val = serde_json::to_value(&structure)?;
      meta["structure"] = struct_val;
    }

    Self::new(&provider_molecule.inchikey,
              &provider_molecule.smiles,
              &provider_molecule.inchi,
              meta)
  }

  // === Getters ===

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

  // === Métodos de dominio ===

  /// Verifica si dos moléculas son la misma (mismo InChIKey)
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_from_parts_valid() {
    let result = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N", // ethanol real InChIKey
                                      "CCO",
                                      "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                      serde_json::json!({}));
    assert!(result.is_ok());
  }

  #[test]
  fn test_from_parts_invalid_inchikey_length() {
    let result = Molecule::from_parts("SHORT", "CCO", "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3", serde_json::json!({}));
    assert!(result.is_err());
    if let Err(DomainError::InvalidFormat { field, .. }) = result {
      assert_eq!(field, "inchikey");
    }
  }

  #[test]
  fn test_from_parts_empty_smiles() {
    let result = Molecule::from_parts("AAAAA-BBBBBBBBB-CCCCCCCCC-P",
                                      "",
                                      "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                      serde_json::json!({}));
    assert!(result.is_err());
    if let Err(DomainError::ValidationError { entity, .. }) = result {
      assert_eq!(entity, "Molecule");
    }
  }

  #[test]
  fn test_is_same() {
    let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N", // ethanol real InChIKey
                                    "CCO",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    serde_json::json!({})).unwrap();

    let mol2 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N", // same InChIKey
                                    "C(C)O",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    serde_json::json!({})).unwrap();

    assert!(mol1.is_same(&mol2));
  }
}
