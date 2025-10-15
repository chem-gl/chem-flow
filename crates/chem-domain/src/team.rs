//! Entidad de dominio para un equipo de usuarios.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::user::User;

/// Representa un equipo o grupo de usuarios.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Team {
  pub id: Uuid,
  pub name: String,
  pub description: Option<String>,
  pub members: Vec<User>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl Team {
  /// Crea un nuevo equipo.
  pub fn new(name: String, description: Option<String>) -> Self {
    let now = Utc::now();
    Self { id: Uuid::new_v4(), name, description, members: Vec::new(), created_at: now, updated_at: now }
  }

  /// Agrega un miembro al equipo.
  pub fn add_member(&mut self, user: User) {
    if !self.members.iter().any(|m| m.id == user.id) {
      self.members.push(user);
      self.updated_at = Utc::now();
    }
  }

  /// Remueve un miembro del equipo.
  pub fn remove_member(&mut self, user_id: &Uuid) {
    if let Some(pos) = self.members.iter().position(|m| &m.id == user_id) {
      self.members.remove(pos);
      self.updated_at = Utc::now();
    }
  }
}
