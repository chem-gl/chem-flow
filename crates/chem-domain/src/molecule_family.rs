//! Entidad MoleculeFamily - Agrupación inmutable de moléculas con hash para
//! integridad
use crate::{DomainError, Molecule};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use uuid::Uuid;
/// Entidad MoleculeFamily
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeFamily {
  id: Uuid,
  name: Option<String>,
  description: Option<String>,
  family_hash: String,
  provenance: Value,
  frozen: bool,
  molecules: Vec<Molecule>,
}
impl MoleculeFamily {
  /// Crea una nueva familia de moléculas
  pub fn new<I>(molecules: I, provenance: Value) -> Result<Self, DomainError>
    where I: IntoIterator<Item = Molecule>
  {
    let mut molecules: Vec<Molecule> = molecules.into_iter().collect();
    if molecules.is_empty() {
      return Err(DomainError::validation("MoleculeFamily", "Una familia molecular no puede estar vacía"));
    }
    let mut seen = HashSet::new();
    molecules.retain(|m| seen.insert(m.inchikey().to_string()));
    let family_hash = Self::calculate_family_hash(&molecules, &provenance);
    Ok(Self { id: Uuid::new_v4(), name: None, description: None, family_hash, provenance, frozen: false, molecules })
  }
  /// Calcula el hash de la familia incluyendo moléculas y provenance (con
  /// serialización canónica)
  fn calculate_family_hash(molecules: &[Molecule], provenance: &Value) -> String {
    let mut inchikeys: Vec<String> = molecules.iter().map(|m| m.inchikey().to_string()).collect();
    inchikeys.sort();
    let combined_inchikeys = inchikeys.join("\n");
    let canon_prov = Self::canonicalize_json(provenance);
    let prov_str = serde_json::to_string(&canon_prov).expect("Falló la serialización canónica de provenance");
    let mut hasher = Sha256::new();
    hasher.update(combined_inchikeys.as_bytes());
    hasher.update(b"\n");
    hasher.update(prov_str.as_bytes());
    format!("{:x}", hasher.finalize())
  }
  /// Serializa el JSON de manera canónica (ordenando claves recursivamente)
  fn canonicalize_json(value: &Value) -> Value {
    match value {
      Value::Object(map) => {
        let mut sorted_map: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in map.iter() {
          sorted_map.insert(k.clone(), Self::canonicalize_json(v));
        }
        Value::Object(Map::from_iter(sorted_map))
      }
      Value::Array(arr) => Value::Array(arr.iter().map(Self::canonicalize_json).collect()),
      _ => value.clone(),
    }
  }
  /// Agrega una molécula a la familia (crea una nueva instancia)
  pub fn add_molecule(&self, molecule: Molecule) -> Result<Self, DomainError> {
    if self.frozen {
      return Err(DomainError::validation("MoleculeFamily", "No se puede modificar una familia congelada"));
    }
    if self.molecules.iter().any(|m| m.inchikey() == molecule.inchikey()) {
      return Err(DomainError::validation("MoleculeFamily",
                                         format!("Molécula ya existe en la familia: {}", molecule.inchikey())));
    }
    let mut new_molecules = self.molecules.clone();
    new_molecules.push(molecule);
    let family_hash = Self::calculate_family_hash(&new_molecules, &self.provenance);
    Ok(Self { id: Uuid::new_v4(),
              name: self.name.clone(),
              description: self.description.clone(),
              family_hash,
              provenance: self.provenance.clone(),
              frozen: self.frozen,
              molecules: new_molecules })
  }
  /// Elimina una molécula de la familia por InChIKey (crea una nueva instancia)
  pub fn remove_molecule(&self, inchikey: &str) -> Result<Self, DomainError> {
    if self.frozen {
      return Err(DomainError::validation("MoleculeFamily", "No se puede modificar una familia congelada"));
    }
    let new_molecules: Vec<Molecule> = self.molecules.iter().filter(|m| m.inchikey() != inchikey).cloned().collect();
    if new_molecules.is_empty() {
      return Err(DomainError::validation("MoleculeFamily", "No se puede eliminar la última molécula de la familia"));
    }
    if new_molecules.len() == self.molecules.len() {
      return Err(DomainError::validation("MoleculeFamily", format!("Molécula no encontrada: {}", inchikey)));
    }
    let family_hash = Self::calculate_family_hash(&new_molecules, &self.provenance);
    Ok(Self { id: Uuid::new_v4(),
              name: self.name.clone(),
              description: self.description.clone(),
              family_hash,
              provenance: self.provenance.clone(),
              frozen: self.frozen,
              molecules: new_molecules })
  }
  /// Congela la familia (impide futuras modificaciones)
  pub fn freeze(&self) -> Self {
    let mut new_family = self.clone();
    new_family.frozen = true;
    new_family
  }
  /// Verifica la integridad del hash de la familia
  pub fn verify_integrity(&self) -> bool {
    Self::calculate_family_hash(&self.molecules, &self.provenance) == self.family_hash
  }
  /// Establece el nombre de la familia (crea una nueva instancia)
  pub fn with_name(&self, name: impl Into<String>) -> Self {
    let mut new_family = self.clone();
    new_family.name = Some(name.into());
    new_family.id = Uuid::new_v4();
    new_family
  }
  /// Establece la descripción de la familia (crea una nueva instancia)
  pub fn with_description(&self, description: impl Into<String>) -> Self {
    let mut new_family = self.clone();
    new_family.description = Some(description.into());
    new_family.id = Uuid::new_v4();
    new_family
  }
  /// Establece el ID de la familia (para persistencia)
  pub fn with_id(&self, id: Uuid) -> Self {
    let mut new_family = self.clone();
    new_family.id = id;
    new_family
  }
  // === Getters ===
  pub fn id(&self) -> Uuid {
    self.id
  }
  pub fn name(&self) -> Option<&str> {
    self.name.as_deref()
  }
  pub fn description(&self) -> Option<&str> {
    self.description.as_deref()
  }
  pub fn family_hash(&self) -> &str {
    &self.family_hash
  }
  pub fn provenance(&self) -> &Value {
    &self.provenance
  }
  pub fn is_frozen(&self) -> bool {
    self.frozen
  }
  pub fn molecules(&self) -> &[Molecule] {
    &self.molecules
  }
  pub fn len(&self) -> usize {
    self.molecules.len()
  }
  pub fn is_empty(&self) -> bool {
    self.molecules.is_empty()
  }
  pub fn contains(&self, inchikey: &str) -> bool {
    self.molecules.iter().any(|m| m.inchikey() == inchikey)
  }
  // === Métodos de dominio ===
  /// Verifica si dos familias son equivalentes (mismo hash)
  pub fn is_equivalent(&self, other: &Self) -> bool {
    self.family_hash == other.family_hash
  }
}
impl<'a> IntoIterator for &'a MoleculeFamily {
  type Item = &'a Molecule;
  type IntoIter = std::slice::Iter<'a, Molecule>;
  fn into_iter(self) -> Self::IntoIter {
    self.molecules.iter()
  }
}
impl IntoIterator for MoleculeFamily {
  type Item = Molecule;
  type IntoIter = std::vec::IntoIter<Molecule>;
  fn into_iter(self) -> Self::IntoIter {
    self.molecules.into_iter()
  }
}
impl fmt::Display for MoleculeFamily {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f,
           "MoleculeFamily(id: {}, name: {}, molecules: {})",
           self.id,
           self.name.as_deref().unwrap_or("sin nombre"),
           self.molecules.len())
  }
}
impl PartialEq for MoleculeFamily {
  fn eq(&self, other: &Self) -> bool {
    self.is_equivalent(other)
  }
}
#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;
  use crate::Molecule;
  #[test]
  fn test_molecule_family_creation() -> Result<(), DomainError> {
    let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                    "CCO",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    json!({}))?;
    let mol2 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-M",
                                    "CCN",
                                    "InChI=1S/C2H7N/c1-2-3/h2-3H2,1H3",
                                    json!({}))?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol1, mol2], provenance)?;
    assert_eq!(family.len(), 2);
    assert!(family.verify_integrity());
    assert!(!family.is_frozen());
    Ok(())
  }
  #[test]
  fn test_molecule_family_duplicates() -> Result<(), DomainError> {
    let mol = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                   "CCO",
                                   "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                   json!({}))?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol.clone(), mol], provenance)?;
    assert_eq!(family.len(), 1);
    Ok(())
  }
  #[test]
  fn test_molecule_family_empty() {
    let provenance = json!({"source": "test"});
    let result = MoleculeFamily::new(Vec::<Molecule>::new(), provenance);
    assert!(result.is_err());
  }
  #[test]
  fn test_add_molecule_to_frozen() -> Result<(), DomainError> {
    let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                    "CCO",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    json!({}))?;
    let provenance = json!({"source": "test"});
    let family = MoleculeFamily::new(vec![mol1.clone()], provenance)?;
    let frozen_family = family.freeze();
    let result = frozen_family.add_molecule(mol1);
    assert!(result.is_err());
    Ok(())
  }
}
