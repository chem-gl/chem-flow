//! Puerto para el repositorio de equipos.

use async_trait::async_trait;
use uuid::Uuid;

use crate::{team::Team, user::User, DomainError};

#[async_trait]
pub trait TeamRepository: Send + Sync {
  async fn save(&self, team: &Team) -> Result<(), DomainError>;
  async fn find_by_id(&self, id: &Uuid) -> Result<Option<Team>, DomainError>;
  async fn delete(&self, id: &Uuid) -> Result<(), DomainError>;
  async fn add_member(&self, team_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
  async fn remove_member(&self, team_id: &Uuid, user_id: &Uuid) -> Result<(), DomainError>;
  async fn get_team_members(&self, team_id: &Uuid) -> Result<Vec<User>, DomainError>;
}
