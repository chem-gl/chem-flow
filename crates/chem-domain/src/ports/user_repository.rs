//! Puerto para el repositorio de usuarios.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{user::User, DomainError};

#[async_trait]
pub trait UserRepository: Send + Sync {
  async fn save(&self, user: &User) -> Result<(), DomainError>;
  async fn find_by_id(&self, id: &Uuid) -> Result<Option<User>, DomainError>;
  async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
  async fn delete(&self, id: &Uuid) -> Result<(), DomainError>;
}
