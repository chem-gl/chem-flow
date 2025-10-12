use chem_domain::{FamilyRepository, InMemoryDomainRepository, Molecule, MoleculeFamily};
use chem_workflow::engine::ChemicalFlowEngine;
use chem_workflow::flows::cadma_flow::steps::admetsa_generated_step6::Step6Input;
use chem_workflow::flows::cadma_flow::steps::admetsa_initial_step4::Step4Input;
use chem_workflow::flows::cadma_flow::steps::admetsa_properties_step2::Step2Input;
use chem_workflow::flows::cadma_flow::steps::common::ADMETSAMethod;
use chem_workflow::flows::cadma_flow::steps::family_reference_step1::Step1Input;
use chem_workflow::flows::cadma_flow::steps::molecule_initial_step3::{GenerationMethod, Step3Input};
use chem_workflow::flows::cadma_flow::steps::substitute_generation_step5::Step5Input;
use chem_workflow::flows::cadma_flow::CadmaFlow;
use chem_workflow::step::StepContext;
use flow::repository::FlowRepository; // bring trait for create_flow
use flow::stubs::InMemoryFlowRepository;
use std::sync::Arc;

// This test runs the full CadmaFlow Steps 1..6 with in-memory repositories.
// Assertions focus on basic chaining: each step produces payload consumable by
// the next.
#[test]
fn full_flow_steps_1_to_6() {
  let flow_repo = Arc::new(InMemoryFlowRepository::new());
  let domain_repo = Arc::new(InMemoryDomainRepository::new());

  // Create flow
  let flow_id = flow_repo.create_flow(Some("e2e".into()), Some("active".into()), serde_json::json!({})).unwrap();
  let flow = CadmaFlow::construct_with_repos(flow_id, flow_repo.clone(), domain_repo.clone());

  // Helper to build context each step
  let ctx = StepContext::new(flow_id, flow_repo.clone(), domain_repo.clone());

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
                              new_family_name: Some("FamE2E".into()),
                              new_family_description: None };
  let s1 = flow.get_step_by_index(0).unwrap();
  let s1_res = s1.execute(&ctx, &serde_json::to_value(&s1_input).unwrap()).unwrap();
  assert!(s1_res.payload.is_object());
  ctx.save_typed_result(s1.name(), s1_res.clone(), -1, None).expect("persist step1");

  // Step2: ADMETSA properties (auto methods)
  let s2_input =
    Step2Input { preferred_methods: vec![ADMETSAMethod::Random4], manual_values: None, method_property_map: None };
  let s2 = flow.get_step_by_index(1).unwrap();
  let s2_res = s2.execute(&ctx, &serde_json::to_value(&s2_input).unwrap()).unwrap();
  assert!(s2_res.payload.is_object());
  ctx.save_typed_result(s2.name(), s2_res.clone(), -1, None).expect("persist step2");

  // Step3: initial molecule generation (manual list of one new molecule)
  let s3_input = Step3Input { method: GenerationMethod::Manual { smiles: "CCCCCCCCO".into() } };
  let s3 = flow.get_step_by_index(2).unwrap();
  let s3_res = s3.execute(&ctx, &serde_json::to_value(&s3_input).unwrap()).unwrap();
  assert!(s3_res.payload.is_object());
  ctx.save_typed_result(s3.name(), s3_res.clone(), -1, None).expect("persist step3");

  // Step4: ADMETSA for initial molecule (reuse Step2 methods)
  let s4_input = Step4Input { override_methods: None, manual_values: None };
  let s4 = flow.get_step_by_index(3).unwrap();
  let s4_res = s4.execute(&ctx, &serde_json::to_value(&s4_input).unwrap()).unwrap();
  assert!(s4_res.payload.is_object());
  ctx.save_typed_result(s4.name(), s4_res.clone(), -1, None).expect("persist step4");

  // Create substitute family (can reuse earlier molecules for simplicity)
  let sub_mol = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N", // ethanol InChIKey
                                     "CCO",
                                     "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                     serde_json::json!({"phase": 2, "source": "test_hardcoded"})).unwrap();
  let sub_family = MoleculeFamily::new(vec![sub_mol.clone()], serde_json::json!({"src":"e2e"})).unwrap();
  let sub_family_id = domain_repo.save_family(sub_family).unwrap();

  // Step5: generate substitutions
  let s5_input = Step5Input { substitute_family_id: Some(sub_family_id),
                              principal_join_points: None,
                              substitute_family_join_points: None,
                              r_substitutes: 1,
                              num_bounds: 1,
                              repeat: false,
                              save_generated: false,
                              include_principal: true,
                              permutation_limit: 0 };
  let s5 = flow.get_step_by_index(4).unwrap();
  let s5_res = s5.execute(&ctx, &serde_json::to_value(&s5_input).unwrap()).unwrap();
  assert!(s5_res.payload.is_object());
  ctx.save_typed_result(s5.name(), s5_res.clone(), -1, None).expect("persist step5");

  // Step6: ADMETSA for generated substitutes
  let s6_input = Step6Input { override_methods: None, manual_values: None };
  let s6 = flow.get_step_by_index(5).unwrap();
  let s6_res = s6.execute(&ctx, &serde_json::to_value(&s6_input).unwrap()).unwrap();
  assert!(s6_res.payload.is_object());
  ctx.save_typed_result(s6.name(), s6_res.clone(), -1, None).expect("persist step6");
}
