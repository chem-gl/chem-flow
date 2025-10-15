use crate::auth::create_jwt;
use crate::errors::ApiError;
use crate::models::{LoginRequest, LoginResponse, RegisterUserRequest, TeamResponse, UserResponse};
use chem_domain::ports::{team_repository::TeamRepository, user_repository::UserRepository};
use chem_domain::{team::Team, user::User};
use chem_persistence::DieselDomainRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserService {
  repo: Arc<DieselDomainRepository>,
}

impl UserService {
  pub fn new(repo: Arc<DieselDomainRepository>) -> Self {
    Self { repo }
  }

  pub async fn register(&self, req: RegisterUserRequest) -> Result<UserResponse, ApiError> {
    let user = User::new(req.name, req.email, req.university, &req.password)?;
    UserRepository::save(&*self.repo, &user).await?;
    Ok(Self::to_user_response(&user))
  }

  pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse, ApiError> {
    let Some(user) = UserRepository::find_by_email(&*self.repo, &req.email).await? else {
      return Err(ApiError::BadRequest("Credenciales inválidas".into()));
    };
    if !user.verify_password(&req.password) {
      return Err(ApiError::BadRequest("Credenciales inválidas".into()));
    }
    let token = create_jwt(user.id).map_err(|_| ApiError::InternalError("No se pudo crear el token".into()))?;
    Ok(LoginResponse { token, token_type: "Bearer".into() })
  }

  pub async fn get_user(&self, id: Uuid) -> Result<UserResponse, ApiError> {
    let Some(user) = UserRepository::find_by_id(&*self.repo, &id).await? else {
      return Err(ApiError::NotFound("Usuario".into()));
    };
    Ok(Self::to_user_response(&user))
  }

  pub async fn delete_user(&self, id: Uuid) -> Result<(), ApiError> {
    Ok(UserRepository::delete(&*self.repo, &id).await?)
  }

  fn to_user_response(u: &User) -> UserResponse {
    UserResponse { id: u.id,
                   name: u.name.clone(),
                   email: u.email.clone(),
                   university: u.university.clone(),
                   created_at: u.created_at.to_rfc3339(),
                   updated_at: u.updated_at.to_rfc3339() }
  }
}

pub struct TeamService {
  repo: Arc<DieselDomainRepository>,
}

impl TeamService {
  pub fn new(repo: Arc<DieselDomainRepository>) -> Self {
    Self { repo }
  }

  pub async fn create(&self, name: String, description: Option<String>) -> Result<TeamResponse, ApiError> {
    let now = chrono::Utc::now();
    let team = Team { id: Uuid::new_v4(), name, description, members: vec![], created_at: now, updated_at: now };
    TeamRepository::save(&*self.repo, &team).await?;
    Ok(TeamResponse { id: team.id,
                      name: team.name,
                      description: team.description,
                      created_at: team.created_at.to_rfc3339(),
                      updated_at: team.updated_at.to_rfc3339(),
                      members: vec![] })
  }

  /// Add a member to a team. `actor` is the authenticated user requesting the operation and
  /// must already be a member of the team to add/remove other members.
  pub async fn add_member(&self, team_id: Uuid, user_id: Uuid, actor: Option<Uuid>) -> Result<(), ApiError> {
    // If actor provided, ensure they are a member of the team
    if let Some(a) = actor {
      let members = self.repo.get_team_members(&team_id).await?;
      let ok = members.iter().any(|m| m.id == a);
      if !ok {
        return Err(ApiError::Unauthorized("not a member of the team".to_string()));
      }
    }
    self.repo.add_member(&team_id, &user_id).await?;
    Ok(())
  }

  pub async fn remove_member(&self, team_id: Uuid, user_id: Uuid, actor: Option<Uuid>) -> Result<(), ApiError> {
    if let Some(a) = actor {
      let members = self.repo.get_team_members(&team_id).await?;
      let ok = members.iter().any(|m| m.id == a);
      if !ok {
        return Err(ApiError::Unauthorized("not a member of the team".to_string()));
      }
    }
    self.repo.remove_member(&team_id, &user_id).await?;
    Ok(())
  }

  pub async fn get(&self, team_id: Uuid) -> Result<TeamResponse, ApiError> {
    let Some(team) = TeamRepository::find_by_id(&*self.repo, &team_id).await? else {
      return Err(ApiError::NotFound("Equipo".into()));
    };
    let members =
      self.repo.get_team_members(&team_id).await?.into_iter().map(|u| UserService::to_user_response(&u)).collect();
    Ok(TeamResponse { id: team.id,
                      name: team.name,
                      description: team.description,
                      created_at: team.created_at.to_rfc3339(),
                      updated_at: team.updated_at.to_rfc3339(),
                      members })
  }
}
