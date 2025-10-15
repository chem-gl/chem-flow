use crate::auth::Claims;
use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::handlers::AppState;
use crate::models::{CreateMoleculeRequest, MoleculeResponse};
use crate::services::MoleculeService;

pub async fn create_molecule(State(state): State<Arc<AppState>>,
                             claims: Claims,
                             Json(req): Json<CreateMoleculeRequest>)
                             -> Result<Json<MoleculeResponse>, ApiError> {
  let svc: &MoleculeService = &state.molecule_service;
  let resp = svc.create_molecule_with_owner(req, Some(claims.sub)).await?;
  Ok(Json(resp))
}

pub async fn list_molecules(State(state): State<Arc<AppState>>) -> Result<Json<Vec<MoleculeResponse>>, ApiError> {
  let svc: &MoleculeService = &state.molecule_service;
  let resp = svc.list_molecules().await?;
  Ok(Json(resp))
}
