use crate::DomainError;
use crate::{Molecule, MoleculeFamily};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// DTO para persistir una propiedad de familia de forma independiente
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedFamilyProperty {
    pub id: Uuid,
    pub family_id: Uuid,
    pub property_type: String,
    pub value: JsonValue,
    pub quality: Option<String>,
    pub preferred: bool,
    pub value_hash: String,
    pub metadata: JsonValue,
}

/// DTO para persistir una propiedad molecular de forma independiente
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OwnedMolecularProperty {
    pub id: Uuid,
    pub molecule_inchikey: String,
    pub property_type: String,
    pub value: JsonValue,
    pub quality: Option<String>,
    pub preferred: bool,
    pub value_hash: String,
    pub metadata: JsonValue,
}

/// Trait que define operaciones de persistencia para el dominio químico.
pub trait DomainRepository: Send + Sync {
    /// Guarda una familia molecular y devuelve su `Uuid`.
    fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;

    /// Recupera una familia por su `Uuid`.
    fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;

    /// Guarda una molécula y devuelve su InChIKey.
    fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;

    /// Obtiene una molécula por su InChIKey.
    fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;

    /// Lista todas las familias (útil para pruebas).
    fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;
    /// Guarda una propiedad de familia (persistible)
    fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError>;
    /// Recupera propiedades de familia por family_id
    fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError>;
    /// Guarda una propiedad molecular
    fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError>;
    /// Recupera propiedades moleculares por inchikey
    fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError>;
    /// Elimina una molécula del repositorio. No permite eliminar si la molécula
    /// forma parte de alguna familia; en ese caso retorna ValidationError.
    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;

    /// Elimina una familia (y sus propiedades y mapeos) del repositorio.
    fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;

    /// Agrega una molécula a una familia existente y persiste la nueva versión
    /// retornando el nuevo `Uuid` de la familia.
    fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError>;

    /// Remueve una molécula de una familia existente y persiste la nueva
    /// versión retornando el nuevo `Uuid` de la familia.
    fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError>;
}

/// Implementación en memoria para tests y desarrollo.
pub struct InMemoryDomainRepository {
    families: Arc<Mutex<HashMap<Uuid, MoleculeFamily>>>,
    molecules: Arc<Mutex<HashMap<String, Molecule>>>,
    family_properties: Arc<Mutex<HashMap<Uuid, OwnedFamilyProperty>>>,
    molecular_properties: Arc<Mutex<HashMap<Uuid, OwnedMolecularProperty>>>,
}

impl InMemoryDomainRepository {
    pub fn new() -> Self {
        Self { families: Arc::new(Mutex::new(HashMap::new())),
               molecules: Arc::new(Mutex::new(HashMap::new())),
               family_properties: Arc::new(Mutex::new(HashMap::new())),
               molecular_properties: Arc::new(Mutex::new(HashMap::new())) }
    }

    // Helper to map poisoned mutex errors into DomainError
    fn lock_map<'a, T>(&'a self, m: &'a Mutex<T>, name: &str) -> Result<std::sync::MutexGuard<'a, T>, DomainError> {
        m.lock()
         .map_err(|e| DomainError::ExternalError(format!("Mutex '{}' poisoned: {}", name, e)))
    }
}

impl DomainRepository for InMemoryDomainRepository {
    fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError> {
        let id = family.id();
        let mut families = self.lock_map(&self.families, "families")?;
        families.insert(id, family);
        Ok(id)
    }

    fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError> {
        let families = self.lock_map(&self.families, "families")?;
        Ok(families.get(id).cloned())
    }

    fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError> {
        let key = molecule.inchikey().to_string();
        let mut molecules = self.lock_map(&self.molecules, "molecules")?;
        molecules.insert(key.clone(), molecule);
        Ok(key)
    }

    fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError> {
        let molecules = self.lock_map(&self.molecules, "molecules")?;
        Ok(molecules.get(inchikey).cloned())
    }

    fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError> {
        let families = self.lock_map(&self.families, "families")?;
        Ok(families.values().cloned().collect())
    }
    fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError> {
        let id = prop.id;
        let mut map = self.lock_map(&self.family_properties, "family_properties")?;
        map.insert(id, prop);
        Ok(id)
    }
    fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError> {
        let map = self.lock_map(&self.family_properties, "family_properties")?;
        Ok(map.values().filter(|p| &p.family_id == family_id).cloned().collect())
    }
    fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError> {
        let id = prop.id;
        let mut map = self.lock_map(&self.molecular_properties, "molecular_properties")?;
        map.insert(id, prop);
        Ok(id)
    }
    fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError> {
        let map = self.lock_map(&self.molecular_properties, "molecular_properties")?;
        Ok(map.values().filter(|p| p.molecule_inchikey == inchikey).cloned().collect())
    }

    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
        // Check families
        let families = self.lock_map(&self.families, "families")?;
        for (_id, fam) in families.iter() {
            if fam.contains(inchikey) {
                return Err(DomainError::ValidationError(format!("No se puede eliminar la molécula {}; pertenece a una familia", inchikey)));
            }
        }
        drop(families);
        let mut molecules = self.lock_map(&self.molecules, "molecules")?;
        molecules.remove(inchikey);
        Ok(())
    }

    fn delete_family(&self, id: &Uuid) -> Result<(), DomainError> {
        let mut families = self.lock_map(&self.families, "families")?;
        families.remove(id);
        // remove family properties
        let mut fps = self.lock_map(&self.family_properties, "family_properties")?;
        fps.retain(|_, v| &v.family_id != id);
        Ok(())
    }

    fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError> {
        let fam_opt = self.get_family(family_id)?;
        let fam = fam_opt.ok_or(DomainError::ValidationError("Familia no encontrada".to_string()))?;
        let new_fam = fam.add_molecule(molecule)?;
        self.save_family(new_fam)
    }

    fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError> {
        let fam_opt = self.get_family(family_id)?;
        let fam = fam_opt.ok_or(DomainError::ValidationError("Familia no encontrada".to_string()))?;
        let new_fam = fam.remove_molecule(inchikey)?;
        self.save_family(new_fam)
    }
}

impl Default for InMemoryDomainRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn save_and_get_family() -> Result<(), DomainError> {
        let repo = InMemoryDomainRepository::new();
        let m1 = crate::Molecule::from_parts("AAAAAAAAAAAAAA-BBBBBBBBBB-C",
                                             "CCO",
                                             "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                             json!({}))?;
        let m2 = crate::Molecule::from_parts("CCCCCCCCCCCCCC-DDDDDDDDDD-E",
                                             "CCN",
                                             "InChI=1S/C2H7N/c1-2-3/h3H,2H2,1H3",
                                             json!({}))?;
        let family = crate::MoleculeFamily::new(vec![m1.clone(), m2.clone()], json!({"test": true}))?;
        let id = repo.save_family(family.clone())?;
        let loaded = repo.get_family(&id)?;
        assert!(loaded.is_some());
        let lf = loaded.unwrap();
        assert_eq!(lf.len(), 2);
        Ok(())
    }

    #[test]
    fn save_and_get_molecule() -> Result<(), DomainError> {
        let repo = InMemoryDomainRepository::new();
        let m = crate::Molecule::from_parts("EEEEEEEEEEEEEE-FFFFFFFFFF-G",
                                            "CCO",
                                            "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                            json!({}))?;
        let key = repo.save_molecule(m.clone())?;
        let loaded = repo.get_molecule(&key)?;
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().inchikey(), m.inchikey());
        Ok(())
    }

    #[test]
    fn save_and_get_properties() -> Result<(), DomainError> {
        let repo = InMemoryDomainRepository::new();
        let m = crate::Molecule::from_parts("HHHHHHHHHHHHHH-IIIIIIIIII-J",
                                            "CCO",
                                            "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                            json!({}))?;
        let _m_key = repo.save_molecule(m.clone())?;

        // Crear familia
        let family = crate::MoleculeFamily::new(vec![m.clone()], json!({"test": true}))?;
        let f_id = repo.save_family(family.clone())?;

        // Guardar propiedad molecular
        let mp = OwnedMolecularProperty { id: uuid::Uuid::new_v4(),
                                          molecule_inchikey: m.inchikey().to_string(),
                                          property_type: "logP".into(),
                                          value: json!(1.23),
                                          quality: Some("high".into()),
                                          preferred: true,
                                          value_hash: "h".into(),
                                          metadata: json!({}) };
        repo.save_molecular_property(mp.clone())?;
        let loaded_mp = repo.get_molecular_properties(m.inchikey())?;
        assert_eq!(loaded_mp.len(), 1);

        // Guardar propiedad de familia
        let fp = OwnedFamilyProperty { id: uuid::Uuid::new_v4(),
                                       family_id: f_id,
                                       property_type: "avg_logP".into(),
                                       value: json!(2.34),
                                       quality: None,
                                       preferred: false,
                                       value_hash: "hf".into(),
                                       metadata: json!({}) };
        repo.save_family_property(fp.clone())?;
        let loaded_fp = repo.get_family_properties(&f_id)?;
        assert_eq!(loaded_fp.len(), 1);

        Ok(())
    }

    #[test]
    fn mutex_poisoning_returns_error() {
        use std::thread;

        let repo = InMemoryDomainRepository::new();

        // Poison the families mutex by panicking while holding the lock in another
        // thread
        let fam_arc = repo.families.clone();
        let handle = thread::spawn(move || {
            let _g = fam_arc.lock().unwrap();
            panic!("force poison");
        });
        let _ = handle.join();

        // Now operations that try to acquire the lock should return
        // DomainError::ExternalError
        let res = repo.list_families();
        assert!(res.is_err());
        if let Err(e) = res {
            match e {
                crate::DomainError::ExternalError(_) => (),
                other => panic!("expected ExternalError, got {:?}", other),
            }
        }
    }
}
