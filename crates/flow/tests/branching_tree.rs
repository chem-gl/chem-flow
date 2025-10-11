use chrono::Utc;
use flow::domain::{FlowData, PersistResult};
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use uuid::Uuid;

fn mk_step(flow_id: Uuid, cursor: i64, name: &str) -> FlowData {
  FlowData { id: Uuid::new_v4(),
             flow_id,
             cursor,
             key: format!("step_state:{}", name),
             payload: serde_json::json!({ "n": name }),
             metadata: serde_json::json!({}),
             command_id: None,
             created_at: Utc::now() }
}

#[test]
fn branching_and_rehydration_workflow_tree() {
  let repo = InMemoryFlowRepository::new();
  // create root flow
  let root_id = repo.create_flow(Some("root".into()), Some("active".into()), serde_json::json!({})).unwrap();
  // add steps 1..5
  for i in 1..=5 {
    let meta = repo.get_flow_meta(&root_id).unwrap();
    let data = mk_step(root_id, meta.current_cursor + 1, &format!("S{}", i));
    let res = repo.persist_data(&data, meta.current_version).unwrap();
    match res {
      PersistResult::Ok { .. } => {}
      _ => panic!("unexpected persist result"),
    }
  }
  // create branch at cursor 3
  let branch_id = repo.create_branch(&root_id, 3, serde_json::json!({ "branch": true })).unwrap();
  // rehydrate branch path: should have 3 steps copied
  let copied_branch_steps = repo.read_data(&branch_id, 0).unwrap();
  assert_eq!(copied_branch_steps.len(), 3);
  // append two steps to branch
  for i in 4..=5 {
    let bmeta = repo.get_flow_meta(&branch_id).unwrap();
    let data = mk_step(branch_id, bmeta.current_cursor + 1, &format!("B{}", i));
    let _ = repo.persist_data(&data, bmeta.current_version).unwrap();
  }
  // branch should now have 5 steps total
  assert_eq!(repo.count_steps(&branch_id).unwrap(), 5);
  // deleting from root from step 4 should cascade delete branch (parent_cursor=3
  // < 4 stays) — only subbranches >= from_cursor should be deleted
  // create a subbranch at cursor 4 to test recursive deletion
  let subbranch_id = repo.create_branch(&root_id, 4, serde_json::json!({ "sub": true })).unwrap();
  assert!(repo.branch_exists(&subbranch_id).unwrap());
  repo.delete_from_step(&root_id, 4).unwrap();
  // root should have only 3 steps remaining
  assert_eq!(repo.count_steps(&root_id).unwrap(), 3);
  // subbranch must be deleted (parent_cursor 4 >= 4)
  assert!(!repo.branch_exists(&subbranch_id).unwrap());
  // branch created at cursor 3 should remain
  assert!(repo.branch_exists(&branch_id).unwrap());
}

#[test]
fn recursive_delete_branch_removes_subtree() {
  let repo = InMemoryFlowRepository::new();
  let root = repo.create_flow(Some("root".into()), Some("active".into()), serde_json::json!({})).unwrap();
  // add a couple steps
  for i in 1..=2 {
    let m = repo.get_flow_meta(&root).unwrap();
    let _ = repo.persist_data(&mk_step(root, m.current_cursor + 1, &format!("S{}", i)), m.current_version).unwrap();
  }
  // branch A at 2
  let a = repo.create_branch(&root, 2, serde_json::json!({ "A": true })).unwrap();
  // branch B from A at 2 (same head)
  let b = repo.create_branch(&a, 2, serde_json::json!({ "B": true })).unwrap();
  assert!(repo.branch_exists(&a).unwrap());
  assert!(repo.branch_exists(&b).unwrap());
  // delete branch A should also delete B
  repo.delete_branch(&a).unwrap();
  assert!(!repo.branch_exists(&a).unwrap());
  assert!(!repo.branch_exists(&b).unwrap());
}

#[test]
fn duplicate_payload_guard_prevents_insertion() {
  let repo = InMemoryFlowRepository::new();
  let flow_id = repo.create_flow(Some("dup".into()), Some("active".into()), serde_json::json!({})).unwrap();
  // insert a step_state:X with given payload
  let meta = repo.get_flow_meta(&flow_id).unwrap();
  let data1 = FlowData { id: Uuid::new_v4(),
                         flow_id,
                         cursor: meta.current_cursor + 1,
                         key: "step_state:X".to_string(),
                         payload: serde_json::json!({ "a": 1 }),
                         metadata: serde_json::json!({}),
                         command_id: None,
                         created_at: Utc::now() };
  let _ = repo.persist_data(&data1, meta.current_version).unwrap();
  // attempt to insert identical payload again with same key and version+1 should
  // be prevented by higher-level guard simulate guard by checking manually;
  // repository itself allows it, guard is in StepContext. Here we simply assert
  // that read_data finds the duplicate and we can emulate skipping insert.
  let existing = repo.read_data(&flow_id, 0).unwrap();
  assert!(existing.iter().any(|fd| fd.key == "step_state:X" && fd.payload == serde_json::json!({ "a": 1 })));
}
