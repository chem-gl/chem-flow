// Clean example: uses in-memory repositories from the workspace to run locally
use chem_domain::InMemoryDomainRepository;
use chem_workflow::engine::WorkflowConfig;
use chem_workflow::flows::CadmaFlow;
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use std::sync::Arc;

fn main() {
    let cfg = WorkflowConfig::default();
    // use in-memory repos for the example so it runs without external services
    let concrete = InMemoryFlowRepository::new();
    let repo: Arc<dyn FlowRepository> = Arc::new(concrete);
    let id = repo.create_flow(Some("cadma-example".into()),
                              Some("active".into()),
                              json!({"workflow_type": "cadma"}))
                 .expect("create flow");

    let domain_concrete = InMemoryDomainRepository::default();
    let domain_arc = Arc::new(domain_concrete);

    // construct flow with repositories
    let mut flow = CadmaFlow::new_with_repos(id, cfg.clone(), repo.clone(), domain_arc.clone());
    println!("Starting CadmaFlow id={}", id);
    flow.execute_step_and_persist().expect("step1");
    println!("After step1: current_step={} status={:?}", flow.current_step(), flow.status());
    flow.execute_step_and_persist().expect("step2");
    println!("After step2: current_step={} status={:?}", flow.current_step(), flow.status());
    let _ = flow.save_snapshot();

    // rehydrate into new instance using same repos
    let mut flow2 = CadmaFlow::new_with_repos(id, cfg, repo.clone(), domain_arc.clone());
    let _ = flow2.rehydrate();
    println!("Rehydrated flow: current_step={} status={:?}",
             flow2.current_step(),
             flow2.status());
}
