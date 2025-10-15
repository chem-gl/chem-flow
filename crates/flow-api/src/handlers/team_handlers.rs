use crate::errors::ApiError;
use crate::models::{CreateTeamRequest, TeamMemberRequest, TeamResponse};
use crate::services::TeamService;
use axum::{
  extract::{Path, State},
  Json,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct TeamState {
  pub team_service: Arc<TeamService>,
}

#[utoipa::path(post,
               path = "/api/teams",
               request_body = CreateTeamRequest,
               responses((status = 201, body = TeamResponse)),
               tag = "Teams")]
pub async fn create_team(State(state): State<TeamState>, Json(req): Json<CreateTeamRequest>)
                         -> Result<Json<TeamResponse>, ApiError> {
  // Creator becomes the initial implicit member if desired (not adding here to keep simple)
  let team = state.team_service.create(req.name, req.description).await?;
  Ok(Json(team))
}

#[utoipa::path(get, path = "/api/teams/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = TeamResponse)), tag = "Teams")]
pub async fn get_team(State(state): State<TeamState>, Path(id): Path<Uuid>) -> Result<Json<TeamResponse>, ApiError> {
  let team = state.team_service.get(id).await?;
  Ok(Json(team))
}

#[utoipa::path(post,
               path = "/api/teams/{id}/members",
               params(("id" = Uuid, Path)),
               request_body = TeamMemberRequest,
               responses((status = 200)),
               tag = "Teams")]
pub async fn add_member(State(state): State<TeamState>,
                        claims: crate::auth::Claims,
                        Path(id): Path<Uuid>,
                        Json(req): Json<TeamMemberRequest>)
                        -> Result<(), ApiError> {
  state.team_service.add_member(id, req.user_id, Some(claims.sub)).await?;
  Ok(())
}

#[utoipa::path(delete,
               path = "/api/teams/{id}/members/{user_id}",
               params(("id" = Uuid, Path), ("user_id" = Uuid, Path)),
               responses((status = 200)),
               tag = "Teams")]
pub async fn remove_member(State(state): State<TeamState>,
                           claims: crate::auth::Claims,
                           Path((id, user_id)): Path<(Uuid, Uuid)>)
                           -> Result<(), ApiError> {
  state.team_service.remove_member(id, user_id, Some(claims.sub)).await?;
  Ok(())
}
