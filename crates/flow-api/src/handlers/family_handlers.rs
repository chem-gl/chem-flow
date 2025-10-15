use crate::auth::Claims;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::handlers::AppState;
use crate::models::AddMoleculeToFamilyRequest;
use crate::models::{CreateFamilyRequest, FamilyResponse};
use axum::extract::Path;

pub async fn create_family(State(state): State<Arc<AppState>>,
                           claims: Claims,
                           Json(req): Json<CreateFamilyRequest>)
                           -> Result<(axum::http::StatusCode, Json<FamilyResponse>), ApiError> {
  let svc = state.family_service.clone();
  let resp = svc.create_family(req, Some(claims.sub)).await?;
  Ok((axum::http::StatusCode::CREATED, Json(resp)))
}

pub async fn add_molecule_to_family(State(state): State<Arc<AppState>>,
                                    claims: Claims,
                                    Path(family_id): Path<uuid::Uuid>,
                                    Json(req): Json<AddMoleculeToFamilyRequest>)
                                    -> Result<Json<uuid::Uuid>, ApiError> {
  // require access to family
  let svc = state.family_service.clone();
  let id = svc.add_molecule(family_id, req.molecule_inchikey.clone(), Some(claims.sub)).await?;
  Ok(Json(id))
}

pub async fn remove_molecule_from_family(State(state): State<Arc<AppState>>,
                                         claims: Claims,
                                         Path((family_id, inchikey)): Path<(uuid::Uuid, String)>)
                                         -> Result<Json<uuid::Uuid>, ApiError> {
  let svc = state.family_service.clone();
  let id = svc.remove_molecule(family_id, &inchikey, Some(claims.sub)).await?;
  Ok(Json(id))
}
