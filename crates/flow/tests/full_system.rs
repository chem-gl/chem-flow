use chrono::Utc;
use flow::domain::FlowData;
use flow::domain::PersistResult;
use flow::engine::FlowEngineConfig;
use flow::stubs::InMemoryFlowRepository;
use flow::FlowEngine;
use flow::FlowRepository;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn full_flow_lifecycle_and_branching() {
  let repo = Arc::new(InMemoryFlowRepository::new());
  let engine = FlowEngine::new(repo.clone(), FlowEngineConfig {});

  // create a flow
  let flow_id = engine.start_flow(Some("root".into()), Some("queued".into()), json!({})).expect("create flow");
  assert!(engine.branch_exists(&flow_id).unwrap());

  // append 5 steps
  for i in 1..=5 {
    let payload = json!({"i": i});
    let metadata = json!({"source": "test"});
    let res = engine.append(flow_id, "Step", payload, metadata, None, (i - 1) as i64).expect("append");
    match res {
      PersistResult::Ok { new_version } => {
        // version should be >=0
        assert!(new_version >= 0);
      }
      PersistResult::Conflict => panic!("unexpected conflict on append {}", i),
    }
  }

  // read items and check count
  let items = engine.get_items(&flow_id, 0).expect("get items");
  assert_eq!(items.len(), 5);
  assert_eq!(engine.count_steps(&flow_id).unwrap(), 5);

  // create a branch from cursor 3
  let child = engine.new_branch(&flow_id, Some("child".into()), Some("queued".into()), 3, json!({})).expect("create branch");
  assert!(engine.branch_exists(&child).unwrap());
  // child should have steps <= cursor 3 plus one BranchCreated record (cursor 4
  // exists but count_steps counts up to current_cursor which equals
  // parent_cursor)
  assert_eq!(engine.count_steps(&child).unwrap(), 3);

  // append a step to child
  let res = engine.append(child, "Step", json!({"child":1}), json!({}), None, 0).expect("append child");
  match res {
    PersistResult::Ok { new_version: _ } => {}
    PersistResult::Conflict => panic!("conflict"),
  }

  // create a grandchild from child at cursor 4 (after BranchCreated)
  let grand = engine.new_branch(&child, Some("grand".into()), Some("queued".into()), 4, json!({})).expect("create grand");
  assert!(engine.branch_exists(&grand).unwrap());

  // delete child branch and ensure grand remains (children become orphaned)
  engine.delete_branch(&child).expect("delete child");
  assert!(!engine.branch_exists(&child).unwrap());
  assert!(engine.branch_exists(&grand).unwrap());

  // delete from step on root: remove steps from cursor 4 and onwards
  engine.delete_from_step(&flow_id, 4).expect("delete from step");
  // root should now have only first 3 steps
  assert_eq!(engine.count_steps(&flow_id).unwrap(), 3);
}

#[test]
fn persist_idempotency_and_conflict() {
  let repo = Arc::new(InMemoryFlowRepository::new());
  let engine = FlowEngine::new(repo.clone(), FlowEngineConfig {});

  let flow_id = engine.start_flow(Some("idemp".into()), Some("queued".into()), json!({})).expect("create");

  // append first step with a command_id
  let cmd = Uuid::new_v4();
  let r1 = engine.append(flow_id, "Step", json!({"v":1}), json!({}), Some(cmd), 0).expect("append1");
  // repeat same command_id: should be idempotent (no duplicate)
  // Use the repository's current_version as expected_version to simulate a client
  // retry with the last known version instead of an outdated one.
  let current_version = repo.get_flow_meta(&flow_id).expect("meta").current_version;
  let r2 = engine.append(flow_id, "Step", json!({"v":1}), json!({}), Some(cmd), current_version).expect("append2");
  match r1 {
    PersistResult::Ok { new_version: v1 } => match r2 {
      PersistResult::Ok { new_version: v2 } => assert_eq!(v1, v2),
      PersistResult::Conflict => panic!("unexpected conflict"),
    },
    PersistResult::Conflict => panic!("unexpected conflict on first append"),
  }

  // Now simulate wrong expected_version -> should return Conflict from persist
  // We craft a FlowData manually with incorrect expected_version
  let meta = repo.get_flow_meta(&flow_id).expect("meta");
  let bad_data = FlowData { id: Uuid::new_v4(),
                            flow_id,
                            cursor: meta.current_cursor + 1,
                            key: "Step".into(),
                            payload: json!({}),
                            metadata: json!({}),
                            command_id: None,
                            created_at: Utc::now() };
  let res = engine.persist_data(&bad_data, meta.current_version + 1).expect("persist_data");
  match res {
    PersistResult::Conflict => {}
    PersistResult::Ok { .. } => panic!("expected conflict due to wrong version"),
  }
}

#[test]
fn snapshots_save_and_load_meta_behaviour() {
  let repo = Arc::new(InMemoryFlowRepository::new());
  let engine = FlowEngine::new(repo.clone(), FlowEngineConfig {});

  let flow_id = engine.start_flow(Some("snap".into()), Some("queued".into()), json!({})).expect("create");
  // append a step
  engine.append(flow_id, "Step", json!({"a":1}), json!({}), None, 0).expect("append");

  // save snapshot metadata
  let snap_id = engine.save_snapshot(&flow_id, 1, "state-1", json!({"k": "v"})).expect("save snapshot");
  // load latest snapshot from repo directly
  let loaded = repo.load_latest_snapshot(&flow_id).expect("load latest");
  assert!(loaded.is_some());
  let meta = loaded.unwrap();
  assert_eq!(meta.id, snap_id);
  assert_eq!(meta.cursor, 1);
}
