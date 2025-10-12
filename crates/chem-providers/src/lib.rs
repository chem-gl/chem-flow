#[cfg(feature = "python")]
use pyo3::PyErr;
use thiserror::Error;
pub mod core;
#[cfg(any(test, feature = "mock_rdkit"))]
pub mod test_utils;
pub use core::Molecule;

// Make the EngineError serializable for better test handling
#[derive(Debug, Error, Clone)]
pub enum EngineError {
  #[cfg(feature = "python")]
  #[error("Error inicializando Python/RDKit: {0}")]
  Init(String),

  #[cfg(feature = "python")]
  #[error("Error obteniendo molécula: {0}")]
  GetMolecule(String),

  #[error("Error de validación: {0}")]
  Validation(String),

  #[error("Error interno: {0}")]
  Internal(String),
}

#[cfg(feature = "python")]
impl From<PyErr> for EngineError {
  fn from(err: PyErr) -> Self {
    EngineError::Internal(err.to_string())
  }
}

// Define trait for mock capability
#[cfg_attr(any(test, feature = "mock_rdkit"), mockall::automock)]
pub trait ChemEngineInterface: Send + Sync {
  fn get_molecule(&self, smiles: &str) -> Result<Molecule, EngineError>;
  fn fuse(&self,
          smiles_a: &str,
          smiles_b: &str,
          atom_a: usize,
          atom_b: usize,
          bond_order: u8)
          -> Result<Molecule, EngineError>;
  fn substitution_points(&self, molecule: &Molecule) -> Vec<usize>;
  fn feasible_bond(&self, mol_a: &Molecule, idx_a: usize, mol_b: &Molecule, idx_b: usize, bond_order: u8) -> bool;
}

/// Motor químico que proporciona acceso a funcionalidades de RDKit vía Python
pub struct ChemEngine {
  _private: (),
}

impl ChemEngine {
  pub fn init() -> Result<Self, EngineError> {
    #[cfg(feature = "python")]
    {
      // Delegamos la inicialización al módulo core
      core::init_python().map_err(|e| EngineError::Init(e.to_string()))?;
    }

    #[cfg(not(feature = "python"))]
    {
      // En modo mock, simplemente creamos la instancia sin inicializar Python
    }

    Ok(Self { _private: () })
  }
}

impl ChemEngineInterface for ChemEngine {
  fn get_molecule(&self, smiles: &str) -> Result<Molecule, EngineError> {
    #[cfg(feature = "python")]
    {
      // Validación básica
      if smiles.trim().is_empty() {
        return Err(EngineError::Validation("SMILES vacío".to_string()));
      }

      // Llamamos al método de Python
      core::get_molecule(smiles).map_err(|e| EngineError::GetMolecule(e.to_string()))
    }

    #[cfg(not(feature = "python"))]
    {
      // En modo mock sin Python, devolvemos una molécula predefinida
      if smiles.trim().is_empty() {
        return Err(EngineError::Validation("SMILES vacío".to_string()));
      }

      Ok(Molecule { inchikey: format!("MOCK-{}", smiles),
                    inchi: format!("InChI=MOCK/{}", smiles),
                    smiles: smiles.to_string(),
                    num_atoms: 1,
                    mol_weight: 12.01,
                    mol_formula: "C".to_string(),
                    structure: Some(core::Structure { atoms: vec![core::Atom { index: 0,
                                                                               atomic_number: 6,
                                                                               symbol: "C".to_string(),
                                                                               implicit_h: 4,
                                                                               total_h: 4 }],
                                                      bonds: vec![],
                                                      substitution_points: vec![0] }) })
    }
  }

  fn fuse(&self,
          smiles_a: &str,
          smiles_b: &str,
          atom_a: usize,
          atom_b: usize,
          bond_order: u8)
          -> Result<Molecule, EngineError> {
    #[cfg(feature = "python")]
    {
      // Validación básica
      if smiles_a.trim().is_empty() || smiles_b.trim().is_empty() {
        return Err(EngineError::Validation("SMILES vacío".to_string()));
      }

      // Llamamos al método de Python
      core::fuse_molecules(smiles_a, smiles_b, atom_a, atom_b, bond_order).map_err(|e| {
                                                                            EngineError::GetMolecule(e.to_string())
                                                                          })
    }

    #[cfg(not(feature = "python"))]
    {
      // En modo mock sin Python, combinamos los SMILES
      if smiles_a.trim().is_empty() || smiles_b.trim().is_empty() {
        return Err(EngineError::Validation("SMILES vacío".to_string()));
      }

      Ok(Molecule { inchikey: format!("MOCK-FUSED-{}-{}", smiles_a, smiles_b),
                    inchi: format!("InChI=MOCK/FUSED/{}-{}", smiles_a, smiles_b),
                    smiles: format!("{}.{}", smiles_a, smiles_b),
                    num_atoms: 2,
                    mol_weight: 24.02,
                    mol_formula: "C2".to_string(),
                    structure: Some(core::Structure { atoms: vec![core::Atom { index: 0,
                                                                               atomic_number: 6,
                                                                               symbol: "C".to_string(),
                                                                               implicit_h: 3,
                                                                               total_h: 3 },
                                                                  core::Atom { index: 1,
                                                                               atomic_number: 6,
                                                                               symbol: "C".to_string(),
                                                                               implicit_h: 3,
                                                                               total_h: 3 },],
                                                      bonds: vec![core::Bond { atom1: 0,
                                                                               atom2: 1,
                                                                               order: 1,
                                                                               is_aromatic: false }],
                                                      substitution_points: vec![0, 1] }) })
    }
  }

  fn substitution_points(&self, mol: &Molecule) -> Vec<usize> {
    mol.structure.as_ref().map(|s| s.substitution_points.clone()).unwrap_or_default()
  }

  /// Heurística simple de factibilidad de enlace: ambos átomos deben tener
  /// al menos 1 hidrógeno disponible (total_h > 0) antes de la unión y el
  /// orden de enlace debe ser 1..=3.
  fn feasible_bond(&self, mol_a: &Molecule, idx_a: usize, mol_b: &Molecule, idx_b: usize, bond_order: u8) -> bool {
    if !(1..=3).contains(&bond_order) {
      return false;
    }
    let Some(str_a) = &mol_a.structure else {
      return true;
    };
    let Some(str_b) = &mol_b.structure else {
      return true;
    };
    if idx_a >= str_a.atoms.len() || idx_b >= str_b.atoms.len() {
      return false;
    }
    let a = &str_a.atoms[idx_a];
    let b = &str_b.atoms[idx_b];
    a.total_h > 0 && b.total_h > 0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_molecule_export() {
    let m = Molecule { smiles: "".to_string(),
                       inchi: "".to_string(),
                       inchikey: "".to_string(),
                       num_atoms: 0,
                       mol_weight: 0.0,
                       mol_formula: "".to_string(),
                       structure: None };
    assert_eq!(m.smiles, "");
    assert_eq!(m.num_atoms, 0);
  }

  #[test]
  #[cfg(not(feature = "python"))]
  fn test_mock_molecule_creation() {
    let engine = ChemEngine::init().expect("Failed to initialize ChemEngine");
    let molecule = engine.get_molecule("CC").expect("Failed to get molecule");
    assert_eq!(molecule.smiles, "CC");
    assert!(molecule.inchikey.contains("MOCK"));
  }

  #[test]
  #[cfg(not(feature = "python"))]
  fn test_mock_molecule_fuse() {
    let engine = ChemEngine::init().expect("Failed to initialize ChemEngine");
    let result = engine.fuse("CC", "CO", 0, 0, 1).expect("Failed to fuse molecules");
    assert_eq!(result.smiles, "CC.CO");
    assert!(result.inchikey.contains("MOCK-FUSED"));
  }
}
