use pyo3::PyErr;
use thiserror::Error;
pub mod core;
pub use core::Molecule;
#[derive(Debug, Error)]
pub enum EngineError {
  #[error("Error inicializando Python/RDKit: {0}")]
  Init(PyErr),
  #[error("Error obteniendo molécula: {0}")]
  GetMolecule(PyErr),
}
/// Motor químico que proporciona acceso a funcionalidades de RDKit vía Python
pub struct ChemEngine {
  _private: (),
}
impl ChemEngine {
  pub fn init() -> Result<Self, EngineError> {
    core::init_python().map_err(EngineError::Init)?;
    Ok(Self { _private: () })
  }
  pub fn get_molecule(&self, smiles: &str) -> Result<Molecule, EngineError> {
    let molecule = core::get_molecule(smiles).map_err(EngineError::GetMolecule)?;
    Ok(molecule)
  }
  pub fn fuse(&self,
              smiles_a: &str,
              smiles_b: &str,
              atom_a: usize,
              atom_b: usize,
              bond_order: u8)
              -> Result<Molecule, EngineError> {
    let molecule = core::fuse_molecules(smiles_a, smiles_b, atom_a, atom_b, bond_order).map_err(EngineError::GetMolecule)?;
    Ok(molecule)
  }
  /// Devuelve los puntos de sustitución (si la estructura está presente)
  pub fn substitution_points(&self, mol: &Molecule) -> Vec<usize> {
    mol.structure.as_ref().map(|s| s.substitution_points.clone()).unwrap_or_default()
  }
  /// Heurística simple de factibilidad de enlace: ambos átomos deben tener
  /// al menos 1 hidrógeno disponible (total_h > 0) antes de la unión y el
  /// orden de enlace debe ser 1..=3.
  pub fn feasible_bond(&self, mol_a: &Molecule, idx_a: usize, mol_b: &Molecule, idx_b: usize, bond_order: u8) -> bool {
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
}
