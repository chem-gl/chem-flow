// test_utils.rs - Utilidades para pruebas con mocks
//
// Este módulo contiene implementaciones en memoria para pruebas
// que permiten ejecutar los tests sin tener que usar RDKit o Python.
use crate::core::{Atom, Bond, Structure};
#[cfg(any(test, feature = "mock_rdkit"))]
use crate::{ChemEngineInterface, EngineError, MockChemEngineInterface, Molecule};
#[cfg(any(test, feature = "mock_rdkit"))]
pub fn create_mock_engine() -> impl ChemEngineInterface {
  let mut mock = MockChemEngineInterface::new();
  // Mock get_molecule
  mock.expect_get_molecule().returning(|smiles| {
                              if smiles.trim().is_empty() {
                                return Err(EngineError::Validation("SMILES vacío".to_string()));
                              }
                              Ok(Molecule { inchikey: format!("MOCK-{}-ABCDEFGHIJ-P", smiles),
                                            inchi: format!("InChI=MOCK/{}", smiles),
                                            smiles: smiles.to_string(),
                                            num_atoms: (smiles.len() as u32).max(1),
                                            mol_weight: 12.01 * (smiles.len() as f64).max(1.0),
                                            mol_formula: format!("C{}", smiles.len().max(1)),
                                            structure: Some(Structure { atoms: vec![Atom { index: 0,
                                                                                           atomic_number: 6,
                                                                                           symbol:
                                                                                             "C".to_string(),
                                                                                           implicit_h: 4,
                                                                                           total_h: 4 }],
                                                                        bonds: vec![],
                                                                        substitution_points: vec![0] }) })
                            });
  // Mock fuse
  mock.expect_fuse().returning(|smiles_a, smiles_b, _, _, _| {
                      if smiles_a.trim().is_empty() || smiles_b.trim().is_empty() {
                        return Err(EngineError::Validation("SMILES vacío".to_string()));
                      }
                      Ok(Molecule { inchikey: format!("MOCK-FUSED-{}-{}-ABCDEFGHIJ-P", smiles_a, smiles_b),
                                    inchi: format!("InChI=MOCK/FUSED/{}-{}", smiles_a, smiles_b),
                                    smiles: format!("{}.{}", smiles_a, smiles_b),
                                    num_atoms: (smiles_a.len() + smiles_b.len()) as u32,
                                    mol_weight: 12.01 * (smiles_a.len() + smiles_b.len()) as f64,
                                    mol_formula: format!("C{}", smiles_a.len() + smiles_b.len()),
                                    structure: Some(Structure { atoms: vec![Atom { index: 0,
                                                                                   atomic_number: 6,
                                                                                   symbol: "C".to_string(),
                                                                                   implicit_h: 3,
                                                                                   total_h: 3 },
                                                                            Atom { index: 1,
                                                                                   atomic_number: 6,
                                                                                   symbol: "C".to_string(),
                                                                                   implicit_h: 3,
                                                                                   total_h: 3 },],
                                                                bonds: vec![Bond { atom1: 0,
                                                                                   atom2: 1,
                                                                                   order: 1,
                                                                                   is_aromatic: false }],
                                                                substitution_points: vec![0, 1] }) })
                    });
  // Mock substitution_points
  mock.expect_substitution_points()
      .returning(|mol| mol.structure.as_ref().map(|s| s.substitution_points.clone()).unwrap_or_default());
  // Mock feasible_bond
  mock.expect_feasible_bond().returning(|mol_a, idx_a, mol_b, idx_b, bond_order| {
                               if !(1..=3).contains(&bond_order) {
                                 return false;
                               }
                               let atoms_a = mol_a.structure.as_ref().map(|s| s.atoms.len()).unwrap_or(0);
                               let atoms_b = mol_b.structure.as_ref().map(|s| s.atoms.len()).unwrap_or(0);
                               // Basic check if indices are valid
                               if idx_a >= atoms_a || idx_b >= atoms_b {
                                 return false;
                               }
                               // Check if atoms have hydrogens
                               let h_a = mol_a.structure
                                              .as_ref()
                                              .and_then(|s| s.atoms.get(idx_a))
                                              .map(|a| a.total_h > 0)
                                              .unwrap_or(true);
                               let h_b = mol_b.structure
                                              .as_ref()
                                              .and_then(|s| s.atoms.get(idx_b))
                                              .map(|a| a.total_h > 0)
                                              .unwrap_or(true);
                               h_a && h_b
                             });
  mock
}
#[cfg(any(test, feature = "mock_rdkit"))]
pub fn setup_test_environment() -> impl ChemEngineInterface {
  create_mock_engine()
}
