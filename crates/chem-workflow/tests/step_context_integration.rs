use chem_domain::InMemoryDomainRepository;
use chem_workflow::flows::cadma_flow::steps::{Step2, Step3};
use chem_workflow::{flows::CadmaFlow, WorkflowError};
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use std::sync::Arc;

#[test]
fn step_context_end_to_end() -> Result<(), WorkflowError> {
  // Crear repositorios en memoria
  let repo = InMemoryFlowRepository::new();
  let domain = InMemoryDomainRepository::new();
  let repo_arc: Arc<dyn flow::repository::FlowRepository> = Arc::new(repo);
  let domain_arc: Arc<dyn chem_domain::DomainRepository> = Arc::new(domain);

  // Crear flow en repo
  let flow_id =
    repo_arc.create_flow(Some("test-flow".into()), Some("created".into()), json!({})).map_err(WorkflowError::from)?;

  // Construir CadmaFlow
  let mut flow = CadmaFlow::new(flow_id, repo_arc.clone(), domain_arc.clone());

  // Ejecutar step1
  let input = json!({});
  let res1 = flow.execute_current_step(&input)?;
  // Persistir resultado de step1
  let persist1 = flow.persist_step_result("step1", res1, -1, None)?;
  match persist1 {
    flow::domain::PersistResult::Ok { new_version: _ } => (),
    flow::domain::PersistResult::Conflict => panic!("unexpected conflict step1"),
  }
  flow.advance_step();

  // Ejecutar step2
  // For step2 we must provide a multiplier in the input so the step can
  // compute saved_value = step1.saved_value * multiplier.
  let input2 = json!({"multiplier": 3});
  let res2 = flow.execute_current_step(&input2)?;
  let persist2 = flow.persist_step_result("step2", res2.clone(), -1, None)?;
  match persist2 {
    flow::domain::PersistResult::Ok { new_version: _ } => (),
    flow::domain::PersistResult::Conflict => panic!("unexpected conflict step2"),
  }
  flow.advance_step();

  // Ejecutar step3, which should read step2 typed payload and compute summary
  let res3 = flow.execute_current_step(&input)?;
  // Recover step3 payload and assert the summary_score is step2.saved_value + 1
  let step2_payload = Step2::recover_from(&res2)?;
  let step3_payload = Step3::recover_from(&res3)?;
  assert_eq!(step3_payload.summary_score, step2_payload.saved_value + 1);

  Ok(())
}
