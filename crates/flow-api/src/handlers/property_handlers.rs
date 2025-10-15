use crate::auth::Claims;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::handlers::AppState;
use crate::models::CreateMolecularPropertyRequest;
use crate::services::PropertyService;

pub async fn create_molecular_property(State(state): State<Arc<AppState>>,
                                       claims: Claims,
                                       Json(req): Json<CreateMolecularPropertyRequest>)
                                       -> Result<Json<()>, ApiError> {
  let svc: &PropertyService = &state.property_service;
  svc.create_molecular_property_with_owner(req, Some(claims.sub)).await?;
  Ok(Json(()))
}
