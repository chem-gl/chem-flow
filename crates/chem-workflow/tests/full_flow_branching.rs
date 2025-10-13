use chem_domain::{FamilyRepository, InMemoryDomainRepository, Molecule, MoleculeFamily, MoleculeReader};
use chem_workflow::flows::cadma_flow::steps::admetsa_generated_step6::{Step6Input, Step6Payload};
use chem_workflow::flows::cadma_flow::steps::admetsa_initial_step4::{Step4Input, Step4Payload};
use chem_workflow::flows::cadma_flow::steps::admetsa_properties_step2::{Step2Input, Step2Payload};
use chem_workflow::flows::cadma_flow::steps::common::{ADMETSAMethod, ADMETSAProperty, ManualValues, MethodPropertyMap};
use chem_workflow::flows::cadma_flow::steps::family_reference_step1::{Step1Input, Step1Payload};
use chem_workflow::flows::cadma_flow::steps::molecule_initial_step3::{GenerationMethod, Step3Input, Step3Payload};
use chem_workflow::flows::cadma_flow::steps::substitute_generation_step5::{Step5Input, Step5Payload};
use chem_workflow::flows::cadma_flow::CadmaFlow;
use chem_workflow::step::StepContext;
use chem_workflow::ChemicalFlowEngine;
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
/// Integration test that executes the full Cadma flow, branching after each
/// step while exercising different method configurations per stage.
#[test]
fn cadma_flow_branching_runs_all_steps() {
  let flow_repo = Arc::new(InMemoryFlowRepository::new());
  let domain_repo = Arc::new(InMemoryDomainRepository::new());
  let flow_id = flow_repo.create_flow(Some("cadma-e2e".into()), Some("active".into()), json!({})).unwrap();
  let flow = CadmaFlow::construct_with_repos(flow_id, flow_repo.clone(), domain_repo.clone());
  let ctx = StepContext::new(flow_id, flow_repo.clone(), domain_repo.clone());
  let mut branch_records: Vec<(Uuid, i64, String)> = Vec::new();
  // Step 1 - create a family from explicit molecules (manual mode)
  let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N", // ethanol InChIKey
                                  "CCO",
                                  "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                  serde_json::json!({"phase": 2, "source": "test_hardcoded"})).unwrap();
  let mol2 = Molecule::from_parts("IJDNQMDRQITEOD-UHFFFAOYSA-N", // butane InChIKey
                                  "CCCC",
                                  "InChI=1S/C4H10/c1-3-4-2/h3-4H2,1-2H3",
                                  serde_json::json!({"phase": 2, "source": "test_hardcoded"})).unwrap();
  let s1_input = Step1Input { families: None,
                              molecules: Some(vec![mol1.clone(), mol2.clone()]),
                              new_family_name: Some("FlowFamily".into()),
                              new_family_description: Some("manual merge".into()) };
  let step1 = flow.get_step_by_index(0).expect("step1");
  let s1_res = step1.execute(&ctx, &serde_json::to_value(&s1_input).unwrap()).unwrap();
  ctx.save_typed_result(step1.name(), s1_res.clone(), -1, None).unwrap();
  let Step1Payload { family_uuid, .. } = serde_json::from_value(s1_res.payload.clone()).unwrap();
  assert!(domain_repo.get_family(&family_uuid).unwrap().is_some(),
          "family should be persisted");
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let branch_id = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"branch":"step1"})).unwrap();
  branch_records.push((branch_id, meta.current_cursor, "Step1".into()));
  // Step 2 - mix manual overrides and random methods
  let mut method_map: MethodPropertyMap = HashMap::new();
  method_map.insert(ADMETSAProperty::LogP, ADMETSAMethod::Manual);
  let mut manual_values: ManualValues = HashMap::new();
  let mut mol1_props = HashMap::new();
  mol1_props.insert(ADMETSAProperty::LogP.as_key(), 1.23);
  manual_values.insert(mol1.smiles().to_string(), mol1_props);
  let s2_input = Step2Input { preferred_methods: vec![ADMETSAMethod::Random4, ADMETSAMethod::Random2],
                              method_property_map: Some(method_map),
                              manual_values: Some(manual_values) };
  let step2 = flow.get_step_by_index(1).expect("step2");
  let s2_res = step2.execute(&ctx, &serde_json::to_value(&s2_input).unwrap()).unwrap();
  ctx.save_typed_result(step2.name(), s2_res.clone(), -1, None).unwrap();
  let Step2Payload { calculated_properties: props_step2, .. } = serde_json::from_value(s2_res.payload.clone()).unwrap();
  assert!(props_step2 > 0, "Step2 should calculate at least one property");
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let branch_id = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"branch":"step2"})).unwrap();
  branch_records.push((branch_id, meta.current_cursor, "Step2".into()));
  // Step 3 - random generation strategy
  let random_candidates = vec!["CCN".to_string(), "CCCO".to_string()];
  let s3_input = Step3Input { method: GenerationMethod::Random { candidates: random_candidates.clone() } };
  let step3 = flow.get_step_by_index(2).expect("step3");
  let s3_res = step3.execute(&ctx, &serde_json::to_value(&s3_input).unwrap()).unwrap();
  ctx.save_typed_result(step3.name(), s3_res.clone(), -1, None).unwrap();
  let Step3Payload { generated_molecules: generated_step3, .. } = serde_json::from_value(s3_res.payload.clone()).unwrap();
  assert_eq!(generated_step3.len(), random_candidates.len());
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let branch_id = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"branch":"step3"})).unwrap();
  branch_records.push((branch_id, meta.current_cursor, "Step3".into()));
  // Step 4 - override methods + manual values for initial molecules
  let mut step4_manuals: ManualValues = HashMap::new();
  let mut manual_props = HashMap::new();
  manual_props.insert(ADMETSAProperty::LogP.as_key(), 7.7);
  step4_manuals.insert(random_candidates[0].clone(), manual_props);
  let s4_input = Step4Input { override_methods: Some(vec![ADMETSAMethod::Random2]), manual_values: Some(step4_manuals) };
  let step4 = flow.get_step_by_index(3).expect("step4");
  let s4_res = step4.execute(&ctx, &serde_json::to_value(&s4_input).unwrap()).unwrap();
  ctx.save_typed_result(step4.name(), s4_res.clone(), -1, None).unwrap();
  let Step4Payload { generated_for: step4_targets, saved_property_ids: props_step4, .. } =
    serde_json::from_value(s4_res.payload.clone()).unwrap();
  assert_eq!(step4_targets.len(), generated_step3.len());
  assert!(!props_step4.is_empty());
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let branch_id = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"branch":"step4"})).unwrap();
  branch_records.push((branch_id, meta.current_cursor, "Step4".into()));
  // Prepare substitute family for Step 5
  let substitute_member = Molecule::from_parts("VNWKTOKETHGBQD-UHFFFAOYSA-N", // methane InChIKey
                                               "C",
                                               "InChI=1S/CH4/h1H4",
                                               serde_json::json!({"phase": 2, "source": "test_hardcoded"})).unwrap();
  let substitute_family = MoleculeFamily::new(vec![substitute_member.clone()], json!({"origin":"branching-test"})).unwrap();
  let substitute_family_id = domain_repo.save_family(substitute_family).unwrap();
  // Step 5 - enable saving and include principal molecules
  let s5_input = Step5Input { substitute_family_id: Some(substitute_family_id),
                              principal_join_points: None,
                              substitute_family_join_points: None,
                              r_substitutes: 1,
                              num_bounds: 1,
                              repeat: true,
                              save_generated: true,
                              include_principal: true,
                              permutation_limit: 4 };
  let step5 = flow.get_step_by_index(4).expect("step5");
  let s5_res = step5.execute(&ctx, &serde_json::to_value(&s5_input).unwrap()).unwrap();
  ctx.save_typed_result(step5.name(), s5_res.clone(), -1, None).unwrap();
  let step5_payload: Step5Payload = serde_json::from_value(s5_res.payload.clone()).unwrap();
  assert!(!step5_payload.generated_molecules.is_empty(),
          "Step5 should generate or retain molecules");
  let saved_payload: Step5Payload =
    ctx.get_step_payload_by_name_typed("SubstituteGenerationStep5").unwrap().expect("persisted step5 payload");
  assert_eq!(saved_payload.generated_count, step5_payload.generated_count);
  for ik in &step5_payload.generated_molecules {
    assert!(domain_repo.get_molecule(ik).unwrap().is_some(),
            "domain should contain generated molecule");
  }
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let branch_id = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"branch":"step5"})).unwrap();
  branch_records.push((branch_id, meta.current_cursor, "Step5".into()));
  // Step 6 - reuse Step5 output with distinct override methods
  let s6_input = Step6Input { override_methods: Some(vec![ADMETSAMethod::Random3]), manual_values: None };
  let step6 = flow.get_step_by_index(5).expect("step6");
  let s6_res = step6.execute(&ctx, &serde_json::to_value(&s6_input).unwrap()).unwrap();
  ctx.save_typed_result(step6.name(), s6_res.clone(), -1, None).unwrap();
  let Step6Payload { calculated_properties: props_step6, generated_for: generated_step6, .. } =
    serde_json::from_value(s6_res.payload.clone()).unwrap();
  assert!(props_step6 > 0);
  assert_eq!(generated_step6.len(), step5_payload.generated_molecules.len());
  let meta = flow_repo.get_flow_meta(&flow_id).unwrap();
  let branch_id = flow_repo.create_branch(&flow_id, meta.current_cursor, json!({"branch":"step6"})).unwrap();
  branch_records.push((branch_id, meta.current_cursor, "Step6".into()));
  // Flow should have one record per step
  assert_eq!(flow_repo.count_steps(&flow_id).unwrap(), 6);
  // Verify branches reflect the snapshot after each step
  let (metas, _) = flow_repo.dump_tables_for_debug().unwrap();
  for (branch_id, cursor, label) in branch_records {
    // label used for assertions in panic messages
    let meta = metas.iter().find(|m| m.id == branch_id).expect("branch metadata");
    assert_eq!(meta.parent_flow_id,
               Some(flow_id),
               "branch {} should link to parent flow",
               label);
    assert_eq!(meta.parent_cursor,
               Some(cursor),
               "branch {} should capture correct cursor",
               label);
    assert_eq!(flow_repo.count_steps(&branch_id).unwrap(),
               cursor,
               "branch {} should copy steps",
               label);
  }
}
