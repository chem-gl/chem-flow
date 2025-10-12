//! Paso 6: calcular propiedades ADMETSA para todas las moléculas generadas en
//! Step5.
//! - Similar a Step4 pero la fuente de moléculas es el payload de Step5.
//! - Reutiliza la configuración (y posibilidad de valores manuales) de Step2.

use crate::errors::WorkflowError;
use crate::flows::cadma_flow::steps::admetsa_properties_step2::Step2Input;
use crate::flows::cadma_flow::steps::common::{ADMETSAMethod, ADMETSAProperty, ManualValues, REQUIRED_PROPERTIES};
use crate::flows::cadma_flow::steps::substitute_generation_step5::Step5Payload;
use crate::impl_workflow_step;
use crate::step::StepContext;
use chem_domain::{Molecule, OwnedMolecularProperty};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step6Input {
  /// Override opcional de métodos preferidos (solo si Step2 usó Manual)
  pub override_methods: Option<Vec<ADMETSAMethod>>,
  /// Valores manuales opcionales (SMILES -> prop_name -> value), solo si Step2
  /// usó Manual
  pub manual_values: Option<ManualValues>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step6Params {
  pub input: Step6Input,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step6Payload {
  pub generated_for: Vec<String>, // inchikeys origen
  pub saved_property_ids: Vec<String>,
  pub calculated_properties: usize,
  pub step_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step6Metadata {
  pub status: String,
  pub parameters: Step6Params,
  pub domain_refs: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ADMETSAGeneratedStep6;

impl ADMETSAGeneratedStep6 {
  fn step2_allowed_manual(&self, ctx: &StepContext) -> Result<(bool, Step2Input), WorkflowError> {
    // Buscar metadata de Step2 (similar a Step4)
    let key = crate::engine::keys::step_state_key("ADMETSAPropertiesStep2");
    let rows = ctx.flow_repo.read_data(&ctx.flow_id, 0)?;
    for fd in rows.iter().rev() {
      if fd.key.eq_ignore_ascii_case(&key) {
        let meta: crate::flows::cadma_flow::steps::admetsa_properties_step2::Step2Metadata =
          serde_json::from_value(fd.metadata.clone())?;
        let input = meta.parameters.input;
        let used_manual = input.manual_values.is_some();
        return Ok((used_manual, input));
      }
    }
    Err(WorkflowError::Validation("No se encontró Step2 para recuperar métodos".into()))
  }

  fn manual_value_for(&self, smiles: &str, prop: ADMETSAProperty, input: &Step6Input) -> Option<f64> {
    let prop_key = format!("{:?}", prop);
    input.manual_values.as_ref().and_then(|mv| mv.get(smiles)).and_then(|pv| pv.get(&prop_key).copied())
  }

  fn choose_method_for_property(&self,
                                prop: ADMETSAProperty,
                                base_input: &Step2Input,
                                override_methods: &Option<Vec<ADMETSAMethod>>)
                                -> ADMETSAMethod {
    if let Some(list) = override_methods {
      if let Some(m) = list.iter().copied().find(|m| m.can_generate(prop)) {
        return m;
      }
    }
    if let Some(map) = &base_input.method_property_map {
      if let Some(&m) = map.get(&prop) {
        return m;
      }
    }
    base_input.preferred_methods.iter().copied().find(|m| m.can_generate(prop)).unwrap_or(ADMETSAMethod::Manual)
  }

  fn compute_for_molecule(&self,
                          molecule: &Molecule,
                          base_input: &Step2Input,
                          step6_input: &Step6Input,
                          allow_manual: bool)
                          -> Result<Vec<OwnedMolecularProperty>, WorkflowError> {
    let smiles = molecule.smiles().to_string();
    let inchikey = molecule.inchikey().to_string();
    let mut props = Vec::with_capacity(REQUIRED_PROPERTIES.len());
    for &prop in &REQUIRED_PROPERTIES {
      let chosen = self.choose_method_for_property(prop, base_input, &step6_input.override_methods);
      let (used_method, value, quality) = if allow_manual {
        if let Some(v) = self.manual_value_for(&smiles, prop, step6_input) {
          (ADMETSAMethod::Manual, v, "manual".to_string())
        } else if chosen == ADMETSAMethod::Manual {
          if let Some(m2) =
            base_input.preferred_methods.iter().copied().find(|m| *m != ADMETSAMethod::Manual && m.can_generate(prop))
          {
            (m2, m2.calculate_mock_value(prop), "calculated".to_string())
          } else {
            (ADMETSAMethod::Manual, 0.0, "manual".to_string())
          }
        } else {
          (chosen, chosen.calculate_mock_value(prop), "calculated".to_string())
        }
      } else if chosen == ADMETSAMethod::Manual {
        if let Some(m2) =
          base_input.preferred_methods.iter().copied().find(|m| *m != ADMETSAMethod::Manual && m.can_generate(prop))
        {
          (m2, m2.calculate_mock_value(prop), "calculated".to_string())
        } else {
          (ADMETSAMethod::Random4, ADMETSAMethod::Random4.calculate_mock_value(prop), "calculated".to_string())
        }
      } else {
        (chosen, chosen.calculate_mock_value(prop), "calculated".to_string())
      };
      let prop_type = format!("{:?}", prop);
      let owned = OwnedMolecularProperty { id: Uuid::new_v4(),
                                           molecule_inchikey: inchikey.clone(),
                                           property_type: prop_type.clone(),
                                           value: serde_json::json!(value),
                                           quality: Some(quality),
                                           preferred: true,
                                           value_hash: format!("{:?}_{}", prop, value),
                                           metadata: serde_json::json!({
                                             "source": "ADMETSAGeneratedStep6",
                                             "method": format!("{:?}", used_method)
                                           }) };
      props.push(owned);
    }
    Ok(props)
  }

  pub fn execute_step(&self, ctx: &StepContext, input: Step6Input) -> Result<crate::step::StepInfo, WorkflowError> {
    // Recuperar payload de Step5
    let step5_payload: Option<Step5Payload> = ctx.get_step_payload_by_name_typed("SubstituteGenerationStep5")?;
    let step5_payload = step5_payload.ok_or_else(|| WorkflowError::Validation("Falta resultado de Step5".into()))?;

    let (allow_manual, base_input) = self.step2_allowed_manual(ctx)?;

    let mut generated_for: Vec<String> = Vec::new();
    let mut saved_ids: Vec<String> = Vec::new();
    for ik in step5_payload.generated_molecules.iter() {
      // ya contiene principal si include_principal
      if let Some(mol) = ctx.domain_repo.get_molecule(ik)? {
        generated_for.push(ik.clone());
        let props = self.compute_for_molecule(&mol, &base_input, &input, allow_manual)?;
        for p in props.into_iter() {
          ctx.domain_repo.save_molecular_property(p.clone())?;
          saved_ids.push(p.id.to_string());
        }
      }
    }
    let payload = Step6Payload { generated_for: generated_for.clone(),
                                 saved_property_ids: saved_ids.clone(),
                                 calculated_properties: saved_ids.len(),
                                 step_result: format!("ok ({} propiedades)", saved_ids.len()) };
    let metadata =
      Step6Metadata { status: "completed".into(), parameters: Step6Params { input }, domain_refs: generated_for };
    Ok(crate::step::StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }
}

impl_workflow_step!(ADMETSAGeneratedStep6,
                    Step6Payload,
                    Step6Metadata,
                    Step6Input,
                    |this_self, ctx, input| { this_self.execute_step(ctx, input) });
