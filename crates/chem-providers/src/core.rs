#[cfg(feature = "python")]
use pyo3::ffi::c_str;
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyModule};
use serde::{Deserialize, Serialize};
#[cfg(feature = "python")]
use std::ffi::CString;
#[cfg(feature = "python")]
use std::sync::OnceLock;
#[cfg(feature = "python")]
static RDKIT_MODULE: OnceLock<Py<PyModule>> = OnceLock::new();
#[cfg(feature = "python")]
pub fn init_python() -> PyResult<()> {
  Python::attach(|py| {
    let code = CString::new(include_str!("../python/rdkit_wrapper.py"))?;
    let module = PyModule::from_code(py, code.as_c_str(), c_str!("rdkit_wrapper.py"), c_str!("rdkit_wrapper"))?;
    // Guardamos el módulo en el OnceLock como Py<PyModule>
    RDKIT_MODULE.set(module.unbind()).ok();
    Ok(())
  })
}
#[cfg(feature = "python")]
fn get_module(py: Python<'_>) -> PyResult<Py<PyModule>> {
  RDKIT_MODULE.get().map(|module| module.clone_ref(py)).ok_or_else(|| {
                                                         PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            "init_python() debe llamarse antes de get_molecule()"
        )
                                                       })
}
#[derive(Debug, Serialize, Deserialize)]
/// Representa una molécula obtenida desde RDKit con propiedades básicas
pub struct Molecule {
  /// Representación SMILES de la molécula
  pub smiles: String,
  /// Representación InChI de la molécula
  pub inchi: String,
  /// Identificador único InChIKey
  pub inchikey: String,
  /// Número de átomos en la molécula
  pub num_atoms: u32,
  /// Peso molecular calculado
  pub mol_weight: f64,
  /// Fórmula molecular
  pub mol_formula: String,
  /// Estructura detallada: átomos, enlaces y puntos de sustitución
  pub structure: Option<Structure>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Structure {
  pub atoms: Vec<Atom>,
  pub bonds: Vec<Bond>,
  #[serde(default)]
  pub substitution_points: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Atom {
  pub index: usize,
  pub atomic_number: u32,
  pub symbol: String,
  pub implicit_h: u32,
  pub total_h: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bond {
  pub atom1: usize,
  pub atom2: usize,
  pub order: u8,
  pub is_aromatic: bool,
}
#[cfg(feature = "python")]
pub fn get_molecule(smiles: &str) -> PyResult<Molecule> {
  Python::attach(|py| {
    let rdkit_py = get_module(py)?;
    let rdkit = rdkit_py.bind(py);
    let binding = rdkit.getattr("molecule_info")?.call1((smiles,))?;
    let info = binding.downcast::<PyDict>()?;
    let json_str: String = py.import("json")?.call_method1("dumps", (info,))?.extract()?;
    let molecule: Molecule = serde_json::from_str(&json_str).map_err(|e| {
                               PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Error de deserialización: {}", e))
                             })?;
    Ok(molecule)
  })
}

/// Fusiona dos moléculas usando RDKit creando un enlace entre atom_a y atom_b.
#[cfg(feature = "python")]
pub fn fuse_molecules(smiles_a: &str, smiles_b: &str, atom_a: usize, atom_b: usize, bond_order: u8) -> PyResult<Molecule> {
  Python::attach(|py| {
    let rdkit_py = get_module(py)?;
    let rdkit = rdkit_py.bind(py);
    let fused_dict = rdkit.getattr("fuse_molecules")?.call1((smiles_a, smiles_b, atom_a, atom_b, bond_order))?;
    let json_str: String = py.import("json")?.call_method1("dumps", (fused_dict,))?.extract()?;
    let molecule: Molecule = serde_json::from_str(&json_str).map_err(|e| {
                               PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Error de deserialización fuse: {}",
                                                                                       e))
                             })?;
    Ok(molecule)
  })
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
  #[cfg(all(feature = "python", not(feature = "mock_rdkit")))]
  fn test_get_molecule() {
    // Este test requiere RDKit real, deshabilitar cuando se usa la feature
    // mock_rdkit
    #[cfg(feature = "mock_rdkit")]
    {
      // Skip: cuando mock_rdkit está activo, no se inicializa Python/RDKit
      return;
    }
    #[cfg(not(feature = "mock_rdkit"))]
    {
      init_python().expect("Fallo al inicializar Python/RDKit");
      let smiles = "CCO"; // Etanol
      let mol = get_molecule(smiles).expect("Fallo al obtener la molécula");
      assert_eq!(mol.smiles, "CCO");
      assert_eq!(mol.num_atoms, 3);
      assert!((mol.mol_weight - 46.07).abs() < 0.1); // Peso molecular
                                                     // aproximado
    }
  }

  #[test]
  #[cfg(all(feature = "python", not(feature = "mock_rdkit")))]
  fn test_structure_atoms_and_bonds() {
    // Este test requiere RDKit real, deshabilitar cuando se usa la feature
    // mock_rdkit
    #[cfg(feature = "mock_rdkit")]
    {
      return;
    }
    #[cfg(not(feature = "mock_rdkit"))]
    {
      // Verify that structure atoms, bonds and substitution points are present
      init_python().expect("Fallo al inicializar Python/RDKit");
      let smiles = "CCO"; // ethanol: C-C-O
      let mol = get_molecule(smiles).expect("Fallo al obtener la molécula");
      let s = mol.structure.expect("Expected structure to be present");
      // atoms count should match num_atoms
      assert_eq!(s.atoms.len() as u32, mol.num_atoms);
      // there should be at least one bond
      assert!(!s.bonds.is_empty(), "Expected at least one bond");
      // check first atom fields
      let a0 = &s.atoms[0];
      assert!(a0.atomic_number > 0);
      assert!(!a0.symbol.is_empty());
      // substitution points should contain at least one heavy atom (indices)
      assert!(!s.substitution_points.is_empty(), "Expected substitution points");
    }
  }

  #[test]
  #[cfg(all(feature = "python", not(feature = "mock_rdkit")))]
  fn test_benzene_aromatic_bonds() {
    // Este test requiere RDKit real, deshabilitar cuando se usa la feature
    // mock_rdkit
    #[cfg(feature = "mock_rdkit")]
    {
      return;
    }
    #[cfg(not(feature = "mock_rdkit"))]
    {
      // Benzene has aromatic bonds; ensure is_aromatic is deserialized
      init_python().expect("Fallo al inicializar Python/RDKit");
      let smiles = "c1ccccc1"; // benzene
      let mol = get_molecule(smiles).expect("Fallo al obtener la molécula");
      let s = mol.structure.expect("Expected structure to be present");
      // should have 6 carbon atoms
      let carbons = s.atoms.iter().filter(|a| a.symbol == "C").count();
      assert_eq!(carbons, 6);
      // at least one bond should be aromatic
      assert!(s.bonds.iter().any(|b| b.is_aromatic), "Expected aromatic bond");
    }
  }
}
