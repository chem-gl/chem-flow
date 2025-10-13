//! Domain Events Module
//!
//! This module contains all domain events - representing significant
//! business occurrences that other parts of the system may need to react to.

use crate::domain::value_objects::{InChIKey, Smiles};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Removed unused macro - all events implement manually

/// Base trait for all domain events
pub trait DomainEvent: Send + Sync + std::fmt::Debug {
  /// Unique identifier for this event instance
  fn event_id(&self) -> Uuid;
  /// When this event occurred
  fn occurred_at(&self) -> DateTime<Utc>;
  /// Type name of the event (for serialization/routing)
  fn event_type(&self) -> &'static str;
  /// Version of the event schema
  fn event_version(&self) -> u32 {
    1
  }
  /// Clone the event as a boxed trait object
  fn clone_event(&self) -> Box<dyn DomainEvent>;
}
/// Event published when a molecule is created
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeCreated {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub molecule_id: Uuid,
  pub inchikey: InChIKey,
  pub smiles: Smiles,
  pub metadata: serde_json::Value,
}
impl MoleculeCreated {
  pub fn new(molecule_id: Uuid, inchikey: InChIKey, smiles: Smiles, metadata: serde_json::Value) -> Self {
    Self { event_id: Uuid::new_v4(), occurred_at: Utc::now(), molecule_id, inchikey, smiles, metadata }
  }
}
impl DomainEvent for MoleculeCreated {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule.created"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
/// Event published when a molecule is updated
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeUpdated {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub molecule_id: Uuid,
  pub inchikey: InChIKey,
  pub previous_version: u64,
  pub new_version: u64,
  pub changes: serde_json::Value,
}
impl MoleculeUpdated {
  pub fn new(molecule_id: Uuid,
             inchikey: InChIKey,
             previous_version: u64,
             new_version: u64,
             changes: serde_json::Value)
             -> Self {
    Self { event_id: Uuid::new_v4(), occurred_at: Utc::now(), molecule_id, inchikey, previous_version, new_version, changes }
  }
}
impl DomainEvent for MoleculeUpdated {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule.updated"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
/// Event published when a molecule is deleted
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeDeleted {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub molecule_id: Uuid,
  pub inchikey: InChIKey,
  pub reason: String,
}
impl MoleculeDeleted {
  pub fn new(molecule_id: Uuid, inchikey: InChIKey, reason: String) -> Self {
    Self { event_id: Uuid::new_v4(), occurred_at: Utc::now(), molecule_id, inchikey, reason }
  }
}
impl DomainEvent for MoleculeDeleted {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule.deleted"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
/// Event published when a molecule family is created
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeFamilyCreated {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub family_id: Uuid,
  pub name: Option<String>,
  pub molecule_count: usize,
  pub molecule_ids: Vec<Uuid>,
}
impl MoleculeFamilyCreated {
  pub fn new(family_id: Uuid, name: Option<String>, molecule_ids: Vec<Uuid>) -> Self {
    let molecule_count = molecule_ids.len();
    Self { event_id: Uuid::new_v4(), occurred_at: Utc::now(), family_id, name, molecule_count, molecule_ids }
  }
}
impl DomainEvent for MoleculeFamilyCreated {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule_family.created"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
/// Event published when a molecule is added to a family
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeAddedToFamily {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub family_id: Uuid,
  pub molecule_id: Uuid,
  pub inchikey: InChIKey,
  pub new_family_size: usize,
}
impl MoleculeAddedToFamily {
  pub fn new(family_id: Uuid, molecule_id: Uuid, inchikey: InChIKey, new_family_size: usize) -> Self {
    Self { event_id: Uuid::new_v4(), occurred_at: Utc::now(), family_id, molecule_id, inchikey, new_family_size }
  }
}
impl DomainEvent for MoleculeAddedToFamily {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule.added_to_family"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
/// Event published when a molecule is removed from a family
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeRemovedFromFamily {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub family_id: Uuid,
  pub molecule_id: Uuid,
  pub inchikey: InChIKey,
  pub new_family_size: usize,
}
impl MoleculeRemovedFromFamily {
  pub fn new(family_id: Uuid, molecule_id: Uuid, inchikey: InChIKey, new_family_size: usize) -> Self {
    Self { event_id: Uuid::new_v4(), occurred_at: Utc::now(), family_id, molecule_id, inchikey, new_family_size }
  }
}
impl DomainEvent for MoleculeRemovedFromFamily {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule.removed_from_family"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
/// Event published when properties are calculated for a molecule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculePropertiesCalculated {
  pub event_id: Uuid,
  pub occurred_at: DateTime<Utc>,
  pub molecule_id: Uuid,
  pub inchikey: InChIKey,
  pub property_count: usize,
  pub calculation_duration_ms: u64,
  pub provider: String,
}
impl MoleculePropertiesCalculated {
  pub fn new(molecule_id: Uuid,
             inchikey: InChIKey,
             property_count: usize,
             calculation_duration_ms: u64,
             provider: String)
             -> Self {
    Self { event_id: Uuid::new_v4(),
           occurred_at: Utc::now(),
           molecule_id,
           inchikey,
           property_count,
           calculation_duration_ms,
           provider }
  }
}
impl DomainEvent for MoleculePropertiesCalculated {
  fn event_id(&self) -> Uuid {
    self.event_id
  }
  fn occurred_at(&self) -> DateTime<Utc> {
    self.occurred_at
  }
  fn event_type(&self) -> &'static str {
    "molecule.properties_calculated"
  }
  fn clone_event(&self) -> Box<dyn DomainEvent> {
    Box::new(self.clone())
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::value_objects::*;
  #[test]
  fn molecule_created_event() -> Result<(), crate::DomainError> {
    let inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")?;
    let smiles = Smiles::new("CCO")?;
    let molecule_id = Uuid::new_v4();
    let event = MoleculeCreated::new(molecule_id, inchikey.clone(), smiles.clone(), serde_json::json!({"test": true}));
    assert_eq!(event.molecule_id, molecule_id);
    assert_eq!(event.inchikey, inchikey);
    assert_eq!(event.smiles, smiles);
    assert_eq!(event.event_type(), "molecule.created");
    Ok(())
  }
  #[test]
  fn events_are_serializable() -> Result<(), crate::DomainError> {
    let inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")?;
    let smiles = Smiles::new("CCO")?;
    let event = MoleculeCreated::new(Uuid::new_v4(), inchikey, smiles, serde_json::json!({}));
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: MoleculeCreated = serde_json::from_str(&json).unwrap();
    assert_eq!(event, deserialized);
    Ok(())
  }
  #[test]
  fn family_events_track_size_changes() {
    let family_id = Uuid::new_v4();
    let molecule_id = Uuid::new_v4();
    let inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N").unwrap();
    let added_event = MoleculeAddedToFamily::new(family_id, molecule_id, inchikey.clone(), 5);
    let removed_event = MoleculeRemovedFromFamily::new(family_id, molecule_id, inchikey, 4);
    assert_eq!(added_event.new_family_size, 5);
    assert_eq!(removed_event.new_family_size, 4);
  }
}
