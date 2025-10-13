//! Test de creación de rama desde punto intermedio con persistencia
//!
//! Este test verifica que podemos crear una rama a partir de un punto
//! intermedio de un flujo, preservando el historial anterior y permitiendo
//! continuar desde ese punto.
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use uuid::Uuid;
#[test]
fn create_branch_from_middle_point() -> Result<(), Box<dyn std::error::Error>> {
  // 1. Crear un repositorio en memoria para las pruebas
  let repo = InMemoryFlowRepository::default();
  // 2. Crear un flujo principal con metadatos
  let flow_id = repo.create_flow(Some("Main Flow".to_string()),
                                 Some("active".to_string()),
                                 json!({
                                     "description": "Main flow for branching test",
                                     "tags": ["main"],
                                     "main_param": "main_value"
                                 }))?;
  // 3. Crear y persistir datos del flujo principal
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
                                  "status": "completed",
                                  "result": "success",
                                  "params": {"param2": "value2"}
                              }),
                              metadata: json!({}),
                              command_id: None,
                              created_at: chrono::Utc::now() };
  let flow_data3 = FlowData { id: Uuid::new_v4(),
                              flow_id,
                              cursor: 3,
                              key: "step3".to_string(),
                              payload: json!({
                                  "status": "in_progress",
                                  "result": "pending",
                                  "params": {"param3": "value3"}
                              }),
                              metadata: json!({}),
                              command_id: None,
                              created_at: chrono::Utc::now() };
  repo.persist_data(&flow_data1, 0)?;
  repo.persist_data(&flow_data2, 1)?;
  repo.persist_data(&flow_data3, 2)?;
  // 4. Crear flujo ramificado con referencia al principal
  let branch_id = repo.create_flow(Some("Branch Flow".to_string()),
                                   Some("active".to_string()),
                                   json!({
                                       "description": "Branched from main flow at step2",
                                       "tags": ["branch"],
                                       "branch_param": "branch_value",
                                       "parent_flow_id": flow_id.to_string(),
                                       "parent_cursor": 2
                                   }))?;
  // 5. Copiar los datos de step1 y step2 a la rama
  let branch_data1 = FlowData { id: Uuid::new_v4(),
                                flow_id: branch_id,
                                cursor: 1,
                                key: "step1".to_string(),
                                payload: flow_data1.payload.clone(),
                                metadata: json!({"copied_from": flow_id.to_string()}),
                                command_id: None,
                                created_at: chrono::Utc::now() };
  let branch_data2 = FlowData { id: Uuid::new_v4(),
                                flow_id: branch_id,
                                cursor: 2,
                                key: "step2".to_string(),
                                payload: flow_data2.payload.clone(),
                                metadata: json!({"copied_from": flow_id.to_string()}),
                                command_id: None,
                                created_at: chrono::Utc::now() };
  repo.persist_data(&branch_data1, 0)?;
  repo.persist_data(&branch_data2, 1)?;
  // 6. Verificar el flujo principal
  let main_meta = repo.get_flow_meta(&flow_id)?;
  let main_steps = repo.read_data(&flow_id, 0)?;
  assert_eq!(main_meta.id, flow_id);
  assert_eq!(main_meta.name, Some("Main Flow".to_string()));
  assert_eq!(main_meta.current_cursor, 3);
  assert_eq!(main_steps.len(), 3);
  // 7. Verificar la rama
  let branch_meta = repo.get_flow_meta(&branch_id)?;
  let branch_steps = repo.read_data(&branch_id, 0)?;
  assert_eq!(branch_meta.id, branch_id);
  assert_eq!(branch_meta.name, Some("Branch Flow".to_string()));
  assert_eq!(branch_meta.current_cursor, 2);
  assert_eq!(branch_steps.len(), 2);
  // Verificar que el parent_flow_id está en los metadatos
  let parent_id = repo.get_meta(&branch_id, "parent_flow_id")?;
  assert_eq!(parent_id.as_str().unwrap(), flow_id.to_string());
  // Verificar que los datos copiados coinciden
  for (i, step) in branch_steps.iter().enumerate() {
    assert_eq!(step.key, main_steps[i].key);
    assert_eq!(step.payload, main_steps[i].payload);
  }
  Ok(())
}
