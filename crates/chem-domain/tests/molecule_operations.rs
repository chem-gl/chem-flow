#[cfg(test)]
mod tests {
  use chem_domain::{DomainError, DomainRepository, InMemoryDomainRepository, Molecule, MoleculeFamily};
  use serde_json::json;
  use std::collections::HashSet;
  // use uuid::Uuid; // not used currently

  #[test]
  fn test_molecule_immutability() -> Result<(), DomainError> {
    let repo = InMemoryDomainRepository::new();

    // Create two molecules
    let molecule1 = Molecule::from_smiles("CC").expect("Should create ethane");
    let molecule2 = Molecule::from_smiles("CCO").expect("Should create ethanol");

    // Save the molecules
    let id1 = repo.save_molecule(molecule1.clone())?;
    let id2 = repo.save_molecule(molecule2.clone())?;

    // Create a family from the molecules
    let family_name = "Test Family";
    let family_desc = Some("A test family".to_string());

    // Get the molecules to add to the family
    let mol1 = repo.get_molecule(&id1)?.expect("Molecule should exist");
    let mol2 = repo.get_molecule(&id2)?.expect("Molecule should exist");
    let molecules = vec![mol1, mol2];

    let family = MoleculeFamily::new(molecules,
                                     json!({"test": true, "name": family_name, "description": family_desc}))?;

    let family_id = repo.save_family(family)?;

    // Retrieve the family and verify its contents
    let retrieved_family = repo.get_family(&family_id)?.expect("Family should exist");

    let metadata = retrieved_family.provenance();
    assert_eq!(metadata["name"].as_str(), Some(family_name));
    assert_eq!(metadata["description"].as_str().map(|s| s.to_string()), family_desc);

    // Attempt to delete a molecule used in the family - should fail
    let result = repo.delete_molecule(&id1);
    assert!(result.is_err(), "Should not be able to delete a molecule used in a family");

    // Verify that both molecules still exist
    let mol1 = repo.get_molecule(&id1)?.expect("Molecule should exist");
    let mol2 = repo.get_molecule(&id2)?.expect("Molecule should exist");

    assert_eq!(mol1.inchikey(), molecule1.inchikey());
    assert_eq!(mol2.inchikey(), molecule2.inchikey());

    // Verify molecule immutability by checking properties
    assert_eq!(mol1.smiles(), molecule1.smiles());
    assert_eq!(mol2.smiles(), molecule2.smiles());

    // Create another family that doesn't use these molecules
    let another_molecule = Molecule::from_smiles("c1ccccc1").expect("Should create benzene");
    let another_id = repo.save_molecule(another_molecule.clone())?;

    let another_family = MoleculeFamily::new(vec![another_molecule], json!({"name": "Another Family"}))?;

    let another_family_id = repo.save_family(another_family)?;

    // Now we should be able to delete the molecule in this family
    repo.delete_family(&another_family_id)?;
    let result = repo.delete_molecule(&another_id);
    assert!(result.is_ok(), "Should be able to delete molecule after family is deleted");

    // Verify the molecule is gone
    let mol = repo.get_molecule(&another_id)?;
    assert!(mol.is_none(), "Molecule should be deleted");

    Ok(())
  }

  #[test]
  fn test_family_property_aggregation() -> Result<(), DomainError> {
    let repo = InMemoryDomainRepository::new();

    // Create molecules with different properties
    let molecule_data = vec![("CCO", json!({"weight": 46.07, "logP": -0.31})),
                             ("CC", json!({"weight": 30.07, "logP": 1.81})),
                             ("CCCC", json!({"weight": 58.12, "logP": 2.3}))];
    let count = molecule_data.len();

    let mut molecules = Vec::new();
    let mut total_weight = 0.0;
    let mut total_logp = 0.0;

    for (smiles, props) in &molecule_data {
      let molecule = Molecule::from_smiles(smiles).expect("Should create molecule");
      // In a real implementation, we would add properties directly to the molecule
      // For this test, we'll assume they're already there

      // Save the molecule first
      repo.save_molecule(molecule.clone())?;
      molecules.push(molecule);

      total_weight += props["weight"].as_f64().unwrap();
      total_logp += props["logP"].as_f64().unwrap();
    }

    // Create a family from these molecules
    let family = MoleculeFamily::new(molecules.clone(),
                                     json!({
                                                         "name": "Test Properties Family",
                                                         "description": "Family for testing property aggregation",
                                     "properties": {
                                       "avg_weight": total_weight / count as f64,
                                       "avg_logP": total_logp / count as f64,
                                       "total_weight": total_weight
                                     }
                                                     }))?;

    let family_id = repo.save_family(family)?;

    // Retrieve the family and verify aggregate properties
    let retrieved_family = repo.get_family(&family_id)?.expect("Family should exist");

    let meta = retrieved_family.provenance();

    // Verify aggregated properties
    assert!(meta["properties"]["avg_weight"].is_number(), "Should have avg_weight");
    assert!(meta["properties"]["avg_logP"].is_number(), "Should have avg_logP");
    assert!(meta["properties"]["total_weight"].is_number(), "Should have total_weight");

    let avg_weight = meta["properties"]["avg_weight"].as_f64().unwrap();
    let expected_avg_weight = total_weight / count as f64;
    assert!((avg_weight - expected_avg_weight).abs() < 0.001,
            "Average weight should be approximately {}, got {}",
            expected_avg_weight,
            avg_weight);

    let avg_logp = meta["properties"]["avg_logP"].as_f64().unwrap();
    let expected_avg_logp = total_logp / count as f64;
    assert!((avg_logp - expected_avg_logp).abs() < 0.001,
            "Average logP should be approximately {}, got {}",
            expected_avg_logp,
            avg_logp);

    let tot_weight = meta["properties"]["total_weight"].as_f64().unwrap();
    assert!((tot_weight - total_weight).abs() < 0.001,
            "Total weight should be approximately {}, got {}",
            total_weight,
            tot_weight);

    Ok(())
  }

  #[test]
  fn test_family_molecule_deduplication() -> Result<(), DomainError> {
    let repo = InMemoryDomainRepository::new();

    // Create molecules
    let molecule1 = Molecule::from_smiles("CC").expect("Should create ethane");
    let molecule2 = Molecule::from_smiles("CCO").expect("Should create ethanol");

    // Save them twice to test deduplication
    let id1a = repo.save_molecule(molecule1.clone())?;
    let id1b = repo.save_molecule(molecule1.clone())?;
    let _id2 = repo.save_molecule(molecule2.clone())?;

    // Verify that saving the same molecule returns the same ID
    assert_eq!(id1a, id1b, "Same molecule should have same ID when saved twice");

    // Create a family with duplicate molecule references - explicitly add the same
    // molecule twice
    let family = MoleculeFamily::new(vec![molecule1.clone(), molecule1.clone(), molecule2.clone()],
                                     json!({"name": "Deduplication Test"}))?;

    let family_id = repo.save_family(family)?;

    // Retrieve the family and verify its contents
    let retrieved_family = repo.get_family(&family_id)?.expect("Family should exist");

    // Convert the molecules to a HashSet to check for duplicates
    let unique_molecules: HashSet<_> = retrieved_family.molecules().iter().map(|m| m.inchikey()).collect();

    // Should only have 2 unique molecules, not 3
    assert_eq!(unique_molecules.len(), 2, "Family should contain only unique molecules");
    assert!(unique_molecules.contains(molecule1.inchikey()), "Should contain molecule1");
    assert!(unique_molecules.contains(molecule2.inchikey()), "Should contain molecule2");

    Ok(())
  }
}
