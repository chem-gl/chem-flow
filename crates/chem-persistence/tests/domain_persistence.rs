use chem_domain::{DomainError, DomainRepository, Molecule, MoleculeFamily};
use chem_persistence::new_domain_repo_from_env;
use serde_json::json;
use uuid::Uuid;
#[test]
fn diesel_domain_persistence_family_lifecycle() {
  // Use a temporary file-backed SQLite DB for tests to avoid URI parsing
  // options problems with different sqlite builds.
  let tmp_path = std::env::temp_dir().join(format!("chem_test_{}.db", Uuid::new_v4()));
  let db_url = tmp_path.to_str().unwrap().to_string();
  std::env::set_var("CHEM_DB_URL", &db_url);
  // If crate was built with the `pg` feature, skip this sqlite-only test at
  // runtime.
  if cfg!(feature = "pg") {
    eprintln!("skipping sqlite-only persistence test because 'pg' feature is enabled");
    return;
  }
  // Create repo from environment (the CHEM_DB_URL we set above). When
  // compiled without `pg` this will use SQLite.
  let repo = new_domain_repo_from_env().expect("failed to create repo");
  // Create two molecules
  // Use valid InChIKey-like strings: 14chars-10chars-1char (total 27 chars)
  let m1 = Molecule::from_parts("ABCDEFGHIJKLMN-OPQRSTUVWX-1",
                                "CCO",
                                "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                json!({})).expect("m1 create");
  let m2 = Molecule::from_parts("ZYXWVUTSRQPONM-MLKJIHGFED-1",
                                "CCN",
                                "InChI=1S/C2H7N/c1-2-3/h3H,2H2,1H3",
                                json!({})).expect("m2 create");
  repo.save_molecule(m1.clone()).expect("save m1");
  repo.save_molecule(m2.clone()).expect("save m2");
  // Create a family from m1
  let fam = MoleculeFamily::new(vec![m1.clone()], json!({"test": true})).expect("family create");
  let id1 = repo.save_family(fam.clone()).expect("save family");
  // Add m2 to family (creates new version)
  let id2 = repo.add_molecule_to_family(&id1, m2.clone()).expect("add molecule to family");
  // Trying to delete m2 should fail because it's referenced by the family version
  // id2
  match repo.delete_molecule(m2.inchikey()) {
    Err(DomainError::ValidationError(_)) => {}
    other => panic!("expected validation error when deleting referenced molecule, got: {:?}", other),
  }
  // Remove m2 from the family version id2 (creates id3)
  let id3 = repo.remove_molecule_from_family(&id2, m2.inchikey()).expect("remove molecule from family");
  // The old version id2 still references m2, so delete_molecule should still fail
  match repo.delete_molecule(m2.inchikey()) {
    Err(DomainError::ValidationError(_)) => {}
    other => panic!("expected validation error after remove (old version exists), got: {:?}", other),
  }
  // Delete the old family version id2 to remove the reference
  repo.delete_family(&id2).expect("delete old family version");
  // Now deletion of m2 should succeed
  repo.delete_molecule(m2.inchikey()).expect("delete molecule after removing family references");
  // Clean up: delete the newest family id3
  repo.delete_family(&id3).expect("delete family id3");
  // Ensure family id3 no longer exists
  let got = repo.get_family(&id3).expect("get family");
  assert!(got.is_none(), "family should have been deleted");
  // Cleanup temporary DB file
  let _ = std::fs::remove_file(tmp_path);
}
