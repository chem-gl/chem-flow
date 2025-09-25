use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
#[test]
fn test_set_get_del_meta_inmemory() {
  let repo = InMemoryFlowRepository::new();
  // create a flow with empty metadata
  let id = repo.create_flow(Some("test-flow".into()), Some("created".into()), json!({})).expect("create");
  // initially no workflow_type
  let v = repo.get_meta(&id, "workflow_type").expect("get_meta");
  assert!(v.is_null(), "expected null for missing key");
  // set workflow_type
  repo.set_meta(&id, "workflow_type", json!("cadma")).expect("set_meta");
  let v = repo.get_meta(&id, "workflow_type").expect("get_meta after set");
  assert_eq!(v.as_str().unwrap(), "cadma");
  // delete it
  repo.del_meta(&id, "workflow_type").expect("del_meta");
  let v = repo.get_meta(&id, "workflow_type").expect("get_meta after del");
  assert!(v.is_null(), "expected null after delete");
}
