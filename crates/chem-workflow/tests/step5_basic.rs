use chem_domain::{DomainRepository, InMemoryDomainRepository, Molecule, MoleculeFamily};
use chem_workflow::flows::cadma_flow::steps::admetsa_initial_step4::{Step4Input, Step4Metadata, Step4Params, Step4Payload};
use chem_workflow::flows::cadma_flow::steps::substitute_generation_step5::{Step5Input, SubstituteGenerationStep5};
use chem_workflow::step::{StepContext, StepInfo};
use flow::repository::FlowRepository; // trait for create_flow
use flow::stubs::InMemoryFlowRepository;
use std::sync::Arc;

#[test]
fn step5_smoke() {
  let flow_repo = Arc::new(InMemoryFlowRepository::new());
  let domain_repo = Arc::new(InMemoryDomainRepository::new());
  let flow_id = flow_repo.create_flow(Some("test".into()), Some("active".into()), serde_json::json!({})).unwrap();
  let ctx = StepContext::new(flow_id, flow_repo.clone(), domain_repo.clone());

  // Create a substitute family with one simple molecule (ethanol) persisted
  let sub_mol = Molecule::from_smiles("CCO").expect("valid smiles");
  let family = MoleculeFamily::new(vec![sub_mol.clone()], serde_json::json!({"origin":"test"})).unwrap();
  let family_id = domain_repo.save_family(family).unwrap();

  // Simulate Step4 payload: generated_for includes one principal molecule
  // (methane) persisted
  let principal = Molecule::from_smiles("C").unwrap();
  let principal_ik = domain_repo.save_molecule(principal).unwrap();
  let step4_payload = Step4Payload { generated_for: vec![principal_ik.clone()],
                                     saved_property_ids: vec![],
                                     calculated_properties: 0,
                                     step_result: "ok".into() };
  let step4_meta = Step4Metadata { status: "ok".into(),
                                   parameters: Step4Params { input: Step4Input { override_methods: None,
                                                                                 manual_values: None } },
                                   domain_refs: vec![principal_ik.clone()] };
  let info = StepInfo { payload: serde_json::to_value(&step4_payload).unwrap(),
                        metadata: serde_json::to_value(&step4_meta).unwrap() };
  ctx.save_typed_result("ADMETSAInitialStep4", info, -1, None).unwrap();

  let step5 = SubstituteGenerationStep5;
  let input = Step5Input { substitute_family_id: Some(family_id),
                           principal_join_points: None,
                           substitute_family_join_points: None,
                           r_substitutes: 1,
                           num_bounds: 1,
                           repeat: true,
                           save_generated: false,
                           include_principal: false,
                           permutation_limit: 0 };

  let res = step5.execute_step(&ctx, input);
  assert!(res.is_ok(), "Step5 should execute successfully: {:?}", res.err());
  let info = res.unwrap();
  assert!(info.payload.is_object());
}
