//! Tests de integración para la API CADMA
//!
//! Valida todos los endpoints con base de datos de prueba

use axum::body::Body;
use axum::http::{Request, StatusCode};
use flow_api::{create_router, AppState, CadmaService};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

/// Helper para crear estado de test con DB temporal
fn create_test_state() -> AppState {
  // Usar repositorios en memoria o SQLite temporal
  let flow_repo = Arc::new(flow::stubs::InMemoryFlowRepository::new());

  // Crear base de datos temporal SQLite para tests
  let temp_db = chem_persistence::test_helpers::create_temp_sqlite_db().unwrap();
  let domain_repo = Arc::new(chem_persistence::DieselDomainRepository::new_with_pool(temp_db.pool.clone()).unwrap());

  let cadma_service = Arc::new(CadmaService::new(flow_repo.clone(), domain_repo.clone()));

  // Family service (test placeholder)
  let family_service = Arc::new(flow_api::services::FamilyService::new(domain_repo.clone()));
  let molecule_service = Arc::new(flow_api::services::MoleculeService::new(domain_repo.clone()));
  let property_service = Arc::new(flow_api::services::PropertyService::new(domain_repo.clone()));
  let user_service = Arc::new(flow_api::services::UserService::new(domain_repo.clone()));
  let team_service = Arc::new(flow_api::services::TeamService::new(domain_repo.clone()));

  AppState { cadma_service, family_service, molecule_service, property_service, user_service, team_service }
}

#[tokio::test]
async fn test_health_check() {
  let state = create_test_state();
  let app = create_router(state);

  let response = app.oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap()).await.unwrap();

  assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_start_cadma_execution() {
  let state = create_test_state();
  let app = create_router(state);

  let request_body = json!({
    "name": "test-execution",
    "metadata": {}
  });

  let response = app.oneshot(Request::builder().method("POST")
                                               .uri("/api/flows/cadma/start")
                                               .header("content-type", "application/json")
                                               .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                                               .unwrap())
                    .await
                    .unwrap();

  assert_eq!(response.status(), StatusCode::CREATED);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

  assert!(body_json["execution_id"].is_string());
  assert_eq!(body_json["status"], "running");
  assert_eq!(body_json["current_step"], 0);
}

#[tokio::test]
async fn test_get_execution_status() {
  let state = create_test_state();

  // Crear una ejecución primero
  let start_req = flow_api::models::StartCadmaRequest { name: Some("test".to_string()), metadata: json!({}) };

  let execution_id = state.cadma_service.start_execution(start_req).unwrap().execution_id;

  let app = create_router(state);

  let response =
    app.oneshot(Request::builder().uri(format!("/api/flows/cadma/{}", execution_id)).body(Body::empty()).unwrap())
       .await
       .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

  assert_eq!(body_json["execution_id"], execution_id.to_string());
  assert!(body_json["status"].is_string());
}

#[tokio::test]
async fn test_list_executions() {
  let state = create_test_state();
  let app = create_router(state);

  let response = app.oneshot(Request::builder().uri("/api/flows/cadma").body(Body::empty()).unwrap()).await.unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

  assert!(body_json["executions"].is_array());
  assert!(body_json["total"].is_number());
}

#[tokio::test]
async fn test_cancel_execution() {
  let state = create_test_state();

  // Crear una ejecución
  let start_req = flow_api::models::StartCadmaRequest { name: Some("test-cancel".to_string()), metadata: json!({}) };

  let execution_id = state.cadma_service.start_execution(start_req).unwrap().execution_id;

  let app = create_router(state);

  let response = app.oneshot(Request::builder().method("DELETE")
                                               .uri(format!("/api/flows/cadma/{}", execution_id))
                                               .body(Body::empty())
                                               .unwrap())
                    .await
                    .unwrap();

  assert_eq!(response.status(), StatusCode::OK);

  let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
  let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

  assert_eq!(body_json["execution_id"], execution_id.to_string());
  assert!(body_json["message"].is_string());
}

#[tokio::test]
async fn test_execute_step_invalid_id() {
  let state = create_test_state();
  let app = create_router(state);

  let fake_id = uuid::Uuid::new_v4();
  let request_body = json!({
    "step_index": 0,
    "payload": {}
  });

  let response = app.oneshot(Request::builder().method("POST")
                                               .uri(format!("/api/flows/cadma/{}/step", fake_id))
                                               .header("content-type", "application/json")
                                               .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                                               .unwrap())
                    .await
                    .unwrap();

  assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
