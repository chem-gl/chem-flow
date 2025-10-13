use base64::Engine;
use chrono::Utc;
use flow::domain::{FlowData, PersistResult};
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use uuid::Uuid;
#[test]
fn snapshot_roundtrip_base64_ptr() {
  let repo = InMemoryFlowRepository::new();
  let fid = repo.create_flow(Some("flow".into()), Some("created".into()), json!({})).unwrap();
  // simulate snapshot stored as base64 in state_ptr
  let state = json!({"current_step": 2, "status": "running"});
  let state_bytes = serde_json::to_vec(&state).unwrap();
  let b64 = base64::engine::general_purpose::STANDARD.encode(state_bytes);
  let snap_id = repo.save_snapshot(&fid, 2, &b64, json!({"fmt":"json"})).unwrap();
  let (bytes, meta) = repo.load_snapshot(&snap_id).unwrap();
  assert_eq!(meta.cursor, 2);
  let b64_back = String::from_utf8(bytes).unwrap();
  let decoded = base64::engine::general_purpose::STANDARD.decode(b64_back.as_bytes()).unwrap();
  let state_back: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
  assert_eq!(state_back["current_step"], 2);
  assert_eq!(state_back["status"], "running");
}
#[test]
fn create_branch_without_artificial_record_and_count() {
  let repo = InMemoryFlowRepository::new();
  let fid = repo.create_flow(Some("p".into()), Some("active".into()), json!({})).unwrap();
  // add two steps
  for i in 1..=2 {
    let data = FlowData { id: Uuid::new_v4(),
                          flow_id: fid,
                          cursor: i,
                          key: format!("step_state:s{}", i),
                          payload: json!({"i": i}),
                          metadata: json!({}),
                          command_id: None,
                          created_at: Utc::now() };
    let meta = repo.get_flow_meta(&fid).unwrap();
    let res = repo.persist_data(&data, meta.current_version).unwrap();
    match res {
      PersistResult::Ok { .. } => {}
      _ => panic!("persist failed"),
    }
  }
  // branch at cursor 2
  let bid = repo.create_branch(&fid, 2, json!({})).unwrap();
  // count should equal parent's step count (no artificial extra records)
  let parent_count = repo.count_steps(&fid).unwrap();
  let branch_count = repo.count_steps(&bid).unwrap();
  assert_eq!(parent_count, 2);
  assert_eq!(branch_count, 2);
}
#[test]
fn set_meta_preserves_other_fields() {
  let repo = InMemoryFlowRepository::new();
  let fid = repo.create_flow(Some("m".into()), Some("s".into()), json!({"a":1,"b":2})).unwrap();
  // change only key `a`
  repo.set_meta(&fid, "a", json!(10)).unwrap();
  // ensure we can read a and b individually
  assert_eq!(repo.get_meta(&fid, "a").unwrap(), json!(10));
  assert_eq!(repo.get_meta(&fid, "b").unwrap(), json!(2));
}
#[test]
fn snapshot_roundtrip_base64_bytes() {
  let repo = InMemoryFlowRepository::new();
  let flow_id = repo.create_flow(Some("t".into()), Some("s".into()), serde_json::json!({})).unwrap();
  // Guardar snapshot como base64 (repo in-mem almacena la cadena en state_ptr)
  let state = serde_json::json!({"hello":"world"});
  let state_bytes = serde_json::to_vec(&state).unwrap();
  let b64 = base64::engine::general_purpose::STANDARD.encode(state_bytes);
  let snap_id = repo.save_snapshot(&flow_id, 0, &b64, serde_json::json!({})).unwrap();
  let (bytes, _meta) = repo.load_snapshot(&snap_id).unwrap();
  let b64_back = String::from_utf8(bytes).unwrap();
  let decoded = base64::engine::general_purpose::STANDARD.decode(b64_back.as_bytes()).unwrap();
  let state_back: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
  assert_eq!(state, state_back);
}
#[test]
fn branch_creation_without_artificial_records() {
  let repo = InMemoryFlowRepository::new();
  let flow_id = repo.create_flow(Some("f".into()), Some("active".into()), serde_json::json!({})).unwrap();
  // persistir dos pasos
  for i in 1..=2 {
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id,
                        cursor: i,
                        key: format!("k{}", i),
                        payload: serde_json::json!({}),
                        metadata: serde_json::json!({}),
                        command_id: None,
                        created_at: chrono::Utc::now() };
    let _ = repo.persist_data(&fd, i - 1).unwrap();
  }
  let branch_id = repo.create_branch(&flow_id, 2, serde_json::json!({})).unwrap();
  // La rama debe tener exactamente 2 pasos copiados (no uno artificial extra)
  let steps = repo.read_data(&branch_id, 0).unwrap();
  assert_eq!(steps.len(), 2);
  // Y su current_cursor debe ser 2 en metadata
  let meta = repo.get_flow_meta(&branch_id).unwrap();
  assert_eq!(meta.current_cursor, 2);
}
#[test]
fn metadata_update_preserves_existing_fields() {
  let repo = InMemoryFlowRepository::new();
  let flow_id = repo.create_flow(Some("m".into()), Some("active".into()), serde_json::json!({})).unwrap();
  // Establecer flow_metadata con dos campos
  repo.set_meta(&flow_id,
                "flow_metadata",
                serde_json::json!({"current_step": 1, "status": "running", "domain_refs": []}))
      .unwrap();
  // Actualizar a objeto con merge manual simulando uso de API de alto nivel
  let mut obj = repo.get_meta(&flow_id, "flow_metadata").unwrap();
  if let Some(map) = obj.as_object_mut() {
    map.insert("current_step".into(), serde_json::json!(2));
  }
  repo.set_meta(&flow_id, "flow_metadata", obj).unwrap();
  let meta = repo.get_meta(&flow_id, "flow_metadata").unwrap();
  assert_eq!(meta.get("current_step").unwrap(), 2);
  assert_eq!(meta.get("status").unwrap(), "running");
}
