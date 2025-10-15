//! Modelos para el control de acceso.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Define el tipo de entidad que tiene acceso a un recurso.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AccessorType {
  User,
  Team,
}

impl fmt::Display for AccessorType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AccessorType::User => write!(f, "user"),
      AccessorType::Team => write!(f, "team"),
    }
  }
}

/// Representa una entrada de control de acceso para una familia de moléculas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeFamilyAccess {
  pub family_id: Uuid,
  pub accessor_id: Uuid,
  pub accessor_type: AccessorType,
}

/// Representa una entrada de control de acceso para una molécula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeAccess {
  pub molecule_id: Uuid,
  pub accessor_id: Uuid,
  pub accessor_type: AccessorType,
}

/// Representa una entrada de control de acceso para un flujo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAccess {
  pub flow_id: Uuid,
  pub accessor_id: Uuid,
  pub accessor_type: AccessorType,
}
