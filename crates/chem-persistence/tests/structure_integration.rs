// TODO Phase 4: Re-enable after PropertyProvider implementation
// This test uses from_smiles which requires external providers (chem-providers)
// In Phase 2, we're isolating the domain from external dependencies
#![cfg(all(feature = "sqlite", not(feature = "postgres")))]
use chem_domain::{FamilyRepository, Molecule, MoleculeFamily, MoleculeReader, MoleculeWriter};
#[test]
fn molecule_and_family_persist_structure() -> Result<(), Box<dyn std::error::Error>> {
  // Create a temporary sqlite-backed repo for testing.
  let (repo, _db_guard) = chem_persistence::new_sqlite_for_test()?;
  // Create a molecule using from_parts (Phase 2: pure domain approach)
  let mol = Molecule::from_simple_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                        "CCO",
                                        "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                        serde_json::json!({
                                          "phase": 2,
                                          "source": "hardcoded",
                                          "structure": {
                                            "atoms": [
                                              {"element": "C", "x": 0.0, "y": 0.0, "z": 0.0},
                                              {"element": "C", "x": 1.5, "y": 0.0, "z": 0.0},
                                              {"element": "O", "x": 2.25, "y": 1.3, "z": 0.0}
                                            ],
                                            "bonds": [
                                              {"from": 0, "to": 1, "order": 1},
                                              {"from": 1, "to": 2, "order": 1}
                                            ],
                                            "substitution_points": [0, 2]
                                          }
                                        }))?; // ethanol
  let inchikey = repo.save_molecule(mol.clone())?;
  // Retrieve the molecule and verify structure in metadata
  let loaded = repo.get_molecule(&inchikey)?.expect("molecule not found");
  let meta = loaded.metadata();
  let structure = meta.get("structure").ok_or("structure missing in metadata")?;
  // atoms should be an array and non-empty
  let atoms = structure.get("atoms").and_then(|a| a.as_array()).ok_or("atoms missing or not array")?;
  assert!(!atoms.is_empty(), "expected atoms array to be non-empty");
  // bonds should be an array
  let bonds = structure.get("bonds").and_then(|b| b.as_array()).ok_or("bonds missing or not array")?;
  assert!(!bonds.is_empty(), "expected bonds array to be non-empty");
  // substitution_points should be present (array)
  let subs =
    structure.get("substitution_points").and_then(|s| s.as_array()).ok_or("substitution_points missing or not array")?;
  assert!(!subs.is_empty(), "expected substitution points to be non-empty");
  // Now create a family containing this molecule and persist it
  let fam = MoleculeFamily::new(vec![loaded.clone()], serde_json::json!({}))?;
  let fam_id = repo.save_family(fam)?;
  // Load the family back and verify molecule includes structure in metadata
  let loaded_fam = repo.get_family(&fam_id)?.expect("family not found");
  assert_eq!(loaded_fam.molecules().len(), 1);
  let family_mol = &loaded_fam.molecules()[0];
  let family_meta = family_mol.metadata();
  assert!(family_meta.get("structure").is_some(),
          "expected structure in family molecule metadata");
  Ok(())
}
