#![cfg(all(feature = "sqlite", not(feature = "postgres")))]
use chem_persistence::test_helpers::create_temp_sqlite_db;
use chem_persistence::DieselFlowRepository;
use chrono::Utc;
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;
struct FlowRepoTestContext {
  snapshot_dir: PathBuf,
  artifact_dir: PathBuf,
}

impl Drop for FlowRepoTestContext {
  fn drop(&mut self) {
    let _ = std::fs::remove_dir_all(&self.snapshot_dir);
    let _ = std::fs::remove_dir_all(&self.artifact_dir);
  }
}

fn setup_repo() -> (DieselFlowRepository, FlowRepoTestContext) {
  let db = create_temp_sqlite_db().expect("failed to create sqlite db");
  let snapshot_dir = std::env::temp_dir().join(format!("chemflow_snapshots_{}", Uuid::new_v4()));
  let artifact_dir = std::env::temp_dir().join(format!("chemflow_artifacts_{}", Uuid::new_v4()));
  let repo =
    DieselFlowRepository::with_pool_and_dirs(db.pool.clone(),
                                             snapshot_dir.to_string_lossy().into_owned(),
                                             artifact_dir.to_string_lossy().into_owned()).expect("failed to initialize \
                                                                                                  repo");
  let ctx = FlowRepoTestContext { snapshot_dir, artifact_dir };
  (repo, ctx)
}
#[test]
fn test_create_and_persist_flow_data_and_branching() {
  let (repo, _ctx) = setup_repo();
  let flow_id = repo.create_flow(Some("mi-flow".into()), Some("running".into()), json!({"k":"v"})).expect("create");
  assert!(repo.branch_exists(&flow_id).unwrap());
  // persist some steps
  let now = Utc::now();
  for i in 1..=3 {
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id,
                        cursor: i,
                        key: "step-result".into(),
                        payload: json!({"i": i}),
                        metadata: json!({}),
                        command_id: None,
                        created_at: now };
    let res = repo.persist_data(&fd, i - 1).expect("persist");
    match res {
      flow::domain::PersistResult::Ok { new_version } => assert!(new_version >= 1),
      flow::domain::PersistResult::Conflict => panic!("conflict"),
    }
  }
  // read back
  let items = repo.read_data(&flow_id, 0).expect("read");
  assert_eq!(items.len(), 3);
  // crear rama en cursor 2
  let branch_id = repo.create_branch(&flow_id, 2, json!({})).expect("branch");
  assert!(repo.branch_exists(&branch_id).unwrap());
  // Verificar que la rama guarda parent_flow_id correctamente
  let meta = repo.get_flow_meta(&branch_id).expect("get meta");
  assert_eq!(meta.parent_flow_id.unwrap(), flow_id);
  // branch should have 2 steps
  let count = repo.count_steps(&branch_id).expect("count");
  assert_eq!(count, 2);
  // --- crear y eliminar rama temporal ---
  let temp_branch = repo.create_branch(&flow_id, 2, json!({})).expect("create temp");
  assert!(repo.branch_exists(&temp_branch).unwrap());
  // Añadir un paso para tener datos
  let now = Utc::now();
  let fd = FlowData { id: Uuid::new_v4(),
                      flow_id: temp_branch,
                      cursor: 3,
                      key: "step".into(),
                      payload: json!({"m":1}),
                      metadata: json!({}),
                      command_id: None,
                      created_at: now };
  let _ = repo.persist_data(&fd, 0).expect("persist temp");
  assert!(repo.branch_exists(&temp_branch).unwrap());
  assert_eq!(repo.count_steps(&temp_branch).unwrap(), 3);
  // eliminar
  repo.delete_branch(&temp_branch).expect("delete branch");
  assert!(!repo.branch_exists(&temp_branch).unwrap());
  // Dump final tables para inspección manual
  let (flows, data) = repo.dump_tables_for_debug().expect("dump");
  println!("flows dump: {} rows", flows.len());
  for f in &flows {
    println!("flow: id={} parent={:?} cursor={} version={}",
             f.id, f.parent_flow_id, f.current_cursor, f.current_version);
  }
  println!("flow_data dump: {} rows", data.len());
  for d in &data {
    println!("data: flow={} cursor={} key={} payload={}",
             d.flow_id, d.cursor, d.key, d.payload);
  }
}
#[test]
fn child_preserves_steps_after_parent_deletion_sqlite() {
  let (repo, _ctx) = setup_repo();
  let parent = repo.create_flow(Some("parent-sql".into()), None, json!({"p":"v"})).expect("create");
  // add steps
  let mut expected = 0i64;
  for i in 1..=5 {
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: parent,
                        cursor: i,
                        key: "Step".into(),
                        payload: json!({"v": i}),
                        metadata: json!({"m": i}),
                        command_id: None,
                        created_at: Utc::now() };
    match repo.persist_data(&fd, expected).expect("persist") {
      flow::domain::PersistResult::Ok { new_version } => expected = new_version,
      _ => panic!("persist failed"),
    }
  }
  // create child clone
  let child = repo.create_branch(&parent, 5, json!({})).expect("branch");
  assert_eq!(repo.count_steps(&child).unwrap(), 5);
  // delete parent; child should remain with its cloned steps
  repo.delete_branch(&parent).expect("delete parent");
  assert!(!repo.branch_exists(&parent).unwrap());
  assert!(repo.branch_exists(&child).unwrap());
  assert_eq!(repo.count_steps(&child).unwrap(), 5);
  let items = repo.read_data(&child, 0).expect("read child");
  assert_eq!(items[0].metadata["m"].as_i64().unwrap(), 1);
}

#[test]
fn delete_from_step_cascades_children_and_truncates_parent_sqlite() {
  let (repo, _ctx) = setup_repo();
  // Create parent and add 5 steps
  let parent = repo.create_flow(Some("parent-del".into()), None, json!({"p":"v"})).expect("create");
  let mut expected = 0i64;
  for i in 1..=5 {
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: parent,
                        cursor: i,
                        key: "Step".into(),
                        payload: json!({"v": i}),
                        metadata: json!({"m": i}),
                        command_id: None,
                        created_at: Utc::now() };
    match repo.persist_data(&fd, expected).expect("persist") {
      flow::domain::PersistResult::Ok { new_version } => expected = new_version,
      _ => panic!("persist failed"),
    }
  }
  // Create two children at different parent_cursor positions
  let child_a = repo.create_branch(&parent, 3, json!({"branch":"a"})).expect("branch a");
  let child_b = repo.create_branch(&parent, 5, json!({"branch":"b"})).expect("branch b");
  assert_eq!(repo.count_steps(&child_a).unwrap(), 3);
  assert_eq!(repo.count_steps(&child_b).unwrap(), 5);
  // Delete from step 4 on the parent: should remove parent steps >=4 and delete
  // child_b (parent_cursor=5)
  repo.delete_from_step(&parent, 4).expect("delete_from_step");
  // Parent now has 3 steps
  assert_eq!(repo.count_steps(&parent).unwrap(), 3);
  // child_a (parent_cursor=3) should remain
  assert!(repo.branch_exists(&child_a).unwrap());
  assert_eq!(repo.count_steps(&child_a).unwrap(), 3);
  // child_b (parent_cursor=5) should be deleted
  assert!(!repo.branch_exists(&child_b).unwrap());
}
