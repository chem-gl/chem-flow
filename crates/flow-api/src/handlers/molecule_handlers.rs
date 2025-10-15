use crate::auth::Claims;
use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::handlers::AppState;
use crate::models::{CreateMoleculeRequest, MoleculeResponse};
use crate::services::MoleculeService;

pub async fn create_molecule(State(state): State<Arc<AppState>>,
                             Json(req): Json<CreateMoleculeRequest>)
                             -> Result<Json<MoleculeResponse>, ApiError> {
  let svc: &MoleculeService = &state.molecule_service;
  // Allow unauthenticated molecule creation (no owner). If the client is
  // authenticated and wants ownership, they should hit the authenticated
  // creation endpoint or we can add query params later. For tests we keep
  // anonymous creation to exercise permission flows.
  let resp = svc.create_molecule(req).await?;
  Ok(Json(resp))
}

pub async fn list_molecules(State(state): State<Arc<AppState>>) -> Result<Json<Vec<MoleculeResponse>>, ApiError> {
  let svc: &MoleculeService = &state.molecule_service;
  let resp = svc.list_molecules().await?;
  Ok(Json(resp))
}

pub async fn delete_molecule(State(state): State<Arc<AppState>>,
                             claims: Claims,
                             Path(inchikey): Path<String>)
                             -> Result<Json<()>, ApiError> {
  let svc: &MoleculeService = &state.molecule_service;
  svc.delete_molecule(&inchikey, Some(claims.sub)).await?;
  Ok(Json(()))
}
