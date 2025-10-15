use crate::errors::ApiError;
use crate::models::{LoginRequest, LoginResponse, RegisterUserRequest, UserResponse};
use crate::services::UserService;
use axum::{extract::State, Json};
use std::sync::Arc;

#[derive(Clone)]
pub struct UserState {
  pub user_service: Arc<UserService>,
}

#[utoipa::path(post,
               path = "/api/auth/register",
               request_body = RegisterUserRequest,
               responses((status = 201, body = UserResponse)),
               tag = "Auth")]
pub async fn register_user(State(state): State<UserState>,
                           Json(req): Json<RegisterUserRequest>)
                           -> Result<Json<UserResponse>, ApiError> {
  let user = state.user_service.register(req).await?;
  Ok(Json(user))
}

#[utoipa::path(post,
               path = "/api/auth/login",
               request_body = LoginRequest,
               responses((status = 200, body = LoginResponse)),
               tag = "Auth")]
pub async fn login(State(state): State<UserState>, Json(req): Json<LoginRequest>) -> Result<Json<LoginResponse>, ApiError> {
  let token = state.user_service.login(req).await?;
  Ok(Json(token))
}
