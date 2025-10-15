use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::handlers::AppState;
use crate::models::{CreateFamilyRequest, FamilyResponse};

#[allow(dead_code)]
pub async fn create_family(State(_state): State<Arc<AppState>>,
                           Json(req): Json<CreateFamilyRequest>)
                           -> Result<Json<FamilyResponse>, ApiError> {
  // Family endpoints are not implemented yet; return a clear error until
  // the service and routes are implemented and AppState contains the
  // necessary services.
  let _ = req; // keep unused-binding silence
  Err(ApiError::InternalError("Not implemented: family endpoints are pending".into()))
}
