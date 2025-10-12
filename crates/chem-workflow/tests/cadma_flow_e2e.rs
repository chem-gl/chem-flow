// Remove unused step imports
use chem_workflow::step::{StepContext, StepInfo};
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use std::sync::Arc;

#[test]
fn cadma_flow_branch_and_rehydrate_and_delete_from_cursor() {
  // Setup in-memory flow repo and a trivial domain repo stub
  let flow_repo = Arc::new(InMemoryFlowRepository::new());
  let domain_repo = Arc::new(chem_domain::InMemoryDomainRepository::new());

  // Create main flow and run initial steps
  let flow_id = FlowRepository::create_flow(&*flow_repo, Some("cadma".into()), Some("running".into()), json!({})).unwrap();
  let ctx = StepContext::new(flow_id, flow_repo.clone(), domain_repo.clone());

  // Simulate step 1 and 2: save some payloads following step_state convention
  let _ = ctx.save_typed_result("STEP1_FAMILY_REF",
                                StepInfo { payload: json!({"family":"stub"}), metadata: json!({}) },
                                -1,
                                None)
             .unwrap();
  let _ = ctx.save_typed_result("STEP2_ADMETSA_PROPS",
                                StepInfo { payload: json!({"props":["logP","TPSA"]}), metadata: json!({}) },
                                -1,
                                None)
             .unwrap();

  // Branch at current cursor (2)
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let child = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"name":"child"})).unwrap();
  assert_eq!(flow_repo.count_steps(&child).unwrap(), meta.current_cursor);

  // Rehydrate on child: get step2 payload by name typed
  let child_ctx = StepContext::new(child, flow_repo.clone(), domain_repo.clone());
  let step2: Option<serde_json::Value> = child_ctx.get_step_payload_by_name_typed("STEP2_ADMETSA_PROPS").unwrap();
  assert!(step2.is_some());

  // Proceed with ADMETSA initial (mocked), save new result on child
  let _ = child_ctx.save_typed_result("STEP4_ADMETSA_INITIAL",
                                      StepInfo { payload: json!({"ok":true}), metadata: json!({}) },
                                      -1,
                                      None)
                   .unwrap();
  assert_eq!(flow_repo.count_steps(&child).unwrap(), meta.current_cursor + 1);

  // Delete from cursor 3 on parent: should keep child (parent_cursor=2 < 3)
  flow_repo.delete_from_step(&flow_id, 3).unwrap();
  assert!(flow_repo.branch_exists(&child).unwrap());
  // Parent now has 2 steps remaining (keeps < 3)
  assert_eq!(flow_repo.count_steps(&flow_id).unwrap(), 2);
}
