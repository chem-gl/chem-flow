//! Entidad de dominio para el usuario.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

/// Representa un usuario en el sistema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
  pub id: Uuid,
  pub name: String,
  pub email: String,
  pub university: Option<String>,
  #[serde(skip_serializing)]
  pub password_hash: String,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl User {
  /// Crea un nuevo usuario con una contraseña hasheada.
  pub fn new(name: String,
             email: String,
             university: Option<String>,
             password_plaintext: &str)
             -> Result<Self, DomainError> {
    if name.is_empty() {
      return Err(DomainError::validation("User", "El nombre no puede estar vacío"));
    }
    if email.is_empty() || !email.contains('@') {
      return Err(DomainError::validation("User", "El email no es válido"));
    }
    if password_plaintext.len() < 8 {
      return Err(DomainError::validation("User", "La contraseña debe tener al menos 8 caracteres"));
    }

    let password_hash = bcrypt::hash(password_plaintext, bcrypt::DEFAULT_COST).map_err(|e| {
                                                                                DomainError::InvalidOperation { operation:
                                                                                                "hash_password".to_string(),
                                                                                              reason:
                                                                                                format!("Error al hashear \
                                                                                                         la contraseña: {}",
                                                                                                        e) }
                                                                              })?;

    let now = Utc::now();
    Ok(Self { id: Uuid::new_v4(), name, email, university, password_hash, created_at: now, updated_at: now })
  }

  /// Verifica si una contraseña coincide con el hash del usuario.
  pub fn verify_password(&self, password_plaintext: &str) -> bool {
    bcrypt::verify(password_plaintext, &self.password_hash).unwrap_or(false)
  }
}
