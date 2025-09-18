use crate::domain_repository::{DomainRepository, InMemoryDomainRepository};
use crate::{Molecule, MoleculeFamily};
use serde_json::json;

pub struct DomainStubs;

impl DomainStubs {
    /// Crea un repositorio en memoria pre-populado con una familia y moléculas
    /// de ejemplo.
    pub fn sample_repo() -> InMemoryDomainRepository {
        let repo = InMemoryDomainRepository::new();

        // Crear dos moléculas simples usando from_parts para evitar dependencia RDKit
        let m1 = Molecule::from_parts("AAAAAAAAAAAAAA-BBBBBBBBBB-C",
                                      "CCO",
                                      "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                      json!({})).unwrap();
        let m2 = Molecule::from_parts("CCCCCCCCCCCCCC-DDDDDDDDDD-E",
                                      "CCN",
                                      "InChI=1S/C2H7N/c1-2-3/h3H,2H2,1H3",
                                      json!({})).unwrap();

        let family = MoleculeFamily::new(vec![m1.clone(), m2.clone()], json!({"source": "stub"})).unwrap();

        let _ = repo.save_molecule(m1);
        let _ = repo.save_molecule(m2);
        let _ = repo.save_family(family);

        repo
    }
}
