//! Test de rehidratación completa de un flujo desde persistencia
//!
//! Este test verifica que podemos recuperar completamente un flujo
//! persistido, incluyendo todo el estado, cursor, payloads y metadatos.
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use uuid::Uuid;
#[test]
fn rehydrate_full_flow() -> Result<(), Box<dyn std::error::Error>> {
  // 1. Crear un repositorio en memoria para las pruebas
  let repo = InMemoryFlowRepository::default();
  // 2. Crear un flujo con metadatos
  let flow_id = repo.create_flow(Some("Test Flow".to_string()),
                                 Some("active".to_string()),
                                 json!({
                                     "description": "Flow for testing rehydration",
                                     "tags": ["test", "rehydration"],
                                     "test_param": "test_value"
                                 }))?;
  // 3. Crear y persistir datos del flujo
  let flow_data1 = FlowData { id: Uuid::new_v4(),
                              flow_id,
                              cursor: 1,
                              key: "step1".to_string(),
                              payload: json!({
                                  "status": "completed",
                                  "result": "success",
                                  "params": {"param1": "value1"}
                              }),
                              metadata: json!({}),
                              command_id: None,
                              created_at: chrono::Utc::now() };
  let flow_data2 = FlowData { id: Uuid::new_v4(),
                              flow_id,
                              cursor: 2,
                              key: "step2".to_string(),
                              payload: json!({
                                  "status": "in_progress",
                                  "result": "pending",
                                  "params": {"param2": "value2"}
                              }),
                              metadata: json!({}),
                              command_id: None,
                              created_at: chrono::Utc::now() };
  repo.persist_data(&flow_data1, 0)?;
  repo.persist_data(&flow_data2, 1)?;
  // 4. Rehidratar el flujo completo
  let flow_meta = repo.get_flow_meta(&flow_id)?;
  let steps = repo.read_data(&flow_id, 0)?;
  // 5. Verificar que todos los datos se han recuperado correctamente
  assert_eq!(flow_meta.id, flow_id);
  assert_eq!(flow_meta.name, Some("Test Flow".to_string()));
  assert_eq!(flow_meta.status, Some("active".to_string()));
  assert_eq!(flow_meta.current_cursor, 2);
  // 6. Verificar metadatos
  let description = repo.get_meta(&flow_id, "description")?;
  assert_eq!(description.as_str().unwrap(), "Flow for testing rehydration");
  let tags = repo.get_meta(&flow_id, "tags")?;
  assert!(tags.is_array());
  assert_eq!(tags.as_array().unwrap().len(), 2);
  assert_eq!(tags[0].as_str().unwrap(), "test");
  assert_eq!(tags[1].as_str().unwrap(), "rehydration");
  // 7. Verificar steps
  assert_eq!(steps.len(), 2);
  let step1 = steps.iter().find(|s| s.key == "step1").unwrap();
  let step2 = steps.iter().find(|s| s.key == "step2").unwrap();
  assert_eq!(step1.cursor, 1);
  assert_eq!(step1.payload["status"], "completed");
  assert_eq!(step1.payload["result"], "success");
  assert_eq!(step1.payload["params"]["param1"], "value1");
  assert_eq!(step2.cursor, 2);
  assert_eq!(step2.payload["status"], "in_progress");
  assert_eq!(step2.payload["result"], "pending");
  assert_eq!(step2.payload["params"]["param2"], "value2");
  Ok(())
}
