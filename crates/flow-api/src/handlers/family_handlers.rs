use crate::auth::Claims;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::handlers::AppState;
use crate::models::{CreateFamilyRequest, FamilyResponse};

pub async fn create_family(State(state): State<Arc<AppState>>,
                           claims: Claims,
                           Json(req): Json<CreateFamilyRequest>)
                           -> Result<(axum::http::StatusCode, Json<FamilyResponse>), ApiError> {
  let svc = state.family_service.clone();
  let resp = svc.create_family(req, Some(claims.sub)).await?;
  Ok((axum::http::StatusCode::CREATED, Json(resp)))
}
