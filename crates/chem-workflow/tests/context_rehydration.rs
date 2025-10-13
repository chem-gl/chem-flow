use chem_domain::InMemoryDomainRepository;
use chem_workflow::step::{StepContext, StepInfo};
use flow::repository::FlowRepository; // bring trait into scope
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use std::sync::Arc;
#[test]
fn step_context_deduplicates_and_retrieves_by_key() {
  // Setup in-memory repos
  let flow_repo = Arc::new(InMemoryFlowRepository::new());
  let domain_repo = Arc::new(InMemoryDomainRepository::new());
  // Create a flow
  let flow_id = FlowRepository::create_flow(&*flow_repo, Some("ctx".into()), Some("running".into()), json!({})).unwrap();
  let ctx = StepContext::new(flow_id, flow_repo.clone(), domain_repo.clone());
  // Save a typed result for step "STEP_ALPHA"
  let info = StepInfo { payload: json!({"foo": 1}), metadata: json!({"bar":"baz"}) };
  let res1 = ctx.save_typed_result("STEP_ALPHA", info.clone(), -1, None).unwrap();
  // Save the same payload again: should deduplicate (no new version)
  let res2 = ctx.save_typed_result("STEP_ALPHA", info.clone(), -1, None).unwrap();
  match (res1, res2) {
    (flow::domain::PersistResult::Ok { new_version: v1 }, flow::domain::PersistResult::Ok { new_version: v2 }) => {
      assert_eq!(v1, v2);
    }
    _ => panic!("unexpected persist results"),
  }
  // Retrieve by name-typed
  let got: Option<serde_json::Value> = ctx.get_step_payload_by_name_typed("STEP_ALPHA").unwrap();
  assert!(got.is_some());
  assert_eq!(got.unwrap()["foo"].as_i64().unwrap(), 1);
  // Ensure get_typed_output_by_type can find latest payload
  let latest: Option<serde_json::Value> = ctx.get_typed_output_by_type::<serde_json::Value>().unwrap();
  assert!(latest.is_some());
}
