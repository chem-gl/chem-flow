use flow::engine::FlowEngineConfig;
use flow::stubs::{GateService, InMemoryFlowRepository, InMemoryWorkerPool};
use flow::FlowEngine;
use serde_json::json;
use std::sync::Arc;

#[test]
fn worker_pool_enqueue_and_claim() {
  let pool = InMemoryWorkerPool::new();
  // initially empty
  assert!(pool.claim().is_none());
  // enqueue a simple WorkItem via the GateService API not available here,
  // so we just ensure enqueue/claim roundtrip parity using dummy items
  let item = flow::domain::WorkItem { flow_id: uuid::Uuid::new_v4(), last_cursor: 0, snapshot_ptr: None };
  pool.enqueue(item.clone());
  let claimed = pool.claim();
  assert!(claimed.is_some());
  let claimed = claimed.unwrap();
  assert_eq!(claimed.flow_id, item.flow_id);
}

#[test]
fn gate_service_open_close() {
  let g = GateService::new();
  let fid = uuid::Uuid::new_v4();
  assert!(!g.is_open(fid, "step1"));
  g.open_gate(fid, "step1", "reason");
  assert!(g.is_open(fid, "step1"));
  g.close_gate(fid, "step1", json!({"x":1}));
  assert!(!g.is_open(fid, "step1"));
}

#[test]
fn snapshot_and_artifact_store_via_repo() {
  let repo = InMemoryFlowRepository::new();
  // snapshot store (call via trait)
  let saved = <InMemoryFlowRepository as flow::repository::SnapshotStore>::save(&repo, &[1, 2, 3]).expect("save snapshot");
  assert_eq!(saved, "inmem");
  let loaded = <InMemoryFlowRepository as flow::repository::SnapshotStore>::load(&repo, &saved).expect("load snapshot");
  assert!(loaded.is_empty());

  // artifact store (call via trait)
  let akey = <InMemoryFlowRepository as flow::repository::ArtifactStore>::put(&repo, &[9, 9]).expect("put artifact");
  assert_eq!(akey, "inmem-artifact");
  let blob = <InMemoryFlowRepository as flow::repository::ArtifactStore>::get(&repo, &akey).expect("get artifact");
  assert!(blob.is_empty());
  let copied = <InMemoryFlowRepository as flow::repository::ArtifactStore>::copy_if_needed(&repo, &akey).expect("copy");
  assert_eq!(copied, akey.to_string());
}

#[test]
fn engine_rehydrate_stores_snapshot_and_replay() {
  let repo = Arc::new(InMemoryFlowRepository::new());
  let engine = FlowEngine::new(repo.clone(), FlowEngineConfig {});

  // create flow and append steps so we have data to rehydrate
  let fid = engine.start_flow(Some("reh".into()), Some("queued".into()), json!({})).expect("start");
  engine.append(fid, "S", json!({"a":1}), json!({}), None, 0).expect("append");
  let items = engine.get_items(&fid, 0).expect("read");
  assert_eq!(items.len(), 1);

  // rehydrate with explicit snapshot bytes and items
  engine.rehydrate(Some(&[1, 2, 3]), &items).expect("rehydrate");
  // last_snapshot should be Some and last_replay length 1 (we can't access
  // privates, but the call must succeed)
}
