// admetsa_initial_step4.rs
//! Paso 4: calcular propiedades ADMETSA para las moléculas iniciales
//! generadas en Step3, reutilizando los métodos definidos en Step2.
//! - Si en Step2 se usó el método Manual, en Step4 se permite override de
//!   métodos y/o valores manuales. De lo contrario, se reutilizan los métodos
//!   de Step2 sin posibilidad de override.

use crate::flows::cadma_flow::steps::admetsa_properties_step2::{Step2Input, Step2Metadata};
use crate::flows::cadma_flow::steps::common::{ADMETSAProperty, ManualValues, REQUIRED_PROPERTIES};
use crate::{errors::WorkflowError, flows::cadma_flow::steps::common::ADMETSAMethod};

use crate::flows::cadma_flow::steps::molecule_initial_step3::Step3Payload;
use crate::impl_workflow_step;
use crate::step::StepContext;
use chem_domain::{Molecule, OwnedMolecularProperty};
use serde::{Deserialize, Serialize};
// no extra imports
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step4Input {
  /// Override opcional de métodos preferidos (solo si Step2 usó Manual)
  pub override_methods: Option<Vec<ADMETSAMethod>>,
  /// Valores manuales opcionales (SMILES -> prop_name -> value), solo si Step2
  /// usó Manual
  pub manual_values: Option<ManualValues>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step4Params {
  pub input: Step4Input,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step4Payload {
  pub generated_for: Vec<String>, // inchikeys
  pub saved_property_ids: Vec<String>,
  pub calculated_properties: usize,
  pub step_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step4Metadata {
  pub status: String,
  pub parameters: Step4Params,
  pub domain_refs: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ADMETSAInitialStep4;

impl ADMETSAInitialStep4 {
  fn step2_allowed_manual(&self, ctx: &StepContext) -> Result<(bool, Step2Input), WorkflowError> {
    // Buscar el FlowData del Step2 por key estándar y leer su metadata para
    // recuperar Step2Input
    let key = crate::engine::keys::step_state_key("ADMETSAPropertiesStep2");
    let rows = ctx.flow_repo.read_data(&ctx.flow_id, 0)?;
    for fd in rows.iter().rev() {
      if fd.key.eq_ignore_ascii_case(&key) {
        // Metadata debe incluir Step2Metadata { parameters: Step2Params { input:
        // Step2Input } }
        let meta: Step2Metadata = serde_json::from_value(fd.metadata.clone())?;
        let input = meta.parameters.input;
        // Consideramos 'usó manual' si se proporcionaron valores manuales en Step2
        let used_manual = input.manual_values.is_some();
        return Ok((used_manual, input));
      }
    }
    Err(WorkflowError::Validation("No se encontró Step2 para recuperar métodos".into()))
  }

  fn choose_method_for_property(&self,
                                prop: ADMETSAProperty,
                                base_input: &Step2Input,
                                override_methods: &Option<Vec<ADMETSAMethod>>)
                                -> ADMETSAMethod {
    // Si hay override, elegir el primer método del override que pueda generar
    if let Some(list) = override_methods {
      if let Some(m) = list.iter().copied().find(|m| m.can_generate(prop)) {
        return m;
      }
    }
    // Prefer map explícito
    if let Some(map) = &base_input.method_property_map {
      if let Some(&m) = map.get(&prop) {
        return m;
      }
    }
    // Luego preferidos de Step2
    base_input.preferred_methods.iter().copied().find(|m| m.can_generate(prop)).unwrap_or(ADMETSAMethod::Manual)
  }

  fn manual_value_for(&self, smiles: &str, prop: ADMETSAProperty, input: &Step4Input) -> Option<f64> {
    let prop_key = format!("{:?}", prop);
    input.manual_values.as_ref().and_then(|mv| mv.get(smiles)).and_then(|pv| pv.get(&prop_key).copied())
  }

  fn compute_for_molecule(&self,
                          molecule: &Molecule,
                          family_id: &Uuid,
                          base_input: &Step2Input,
                          step4_input: &Step4Input,
                          allow_manual: bool)
                          -> Result<Vec<OwnedMolecularProperty>, WorkflowError> {
    let smiles = molecule.smiles().to_string();
    let inchikey = molecule.inchikey().to_string();
    let mut props = Vec::with_capacity(REQUIRED_PROPERTIES.len());

    for &prop in &REQUIRED_PROPERTIES {
      // Elegir método
      let chosen = self.choose_method_for_property(prop, base_input, &step4_input.override_methods);
      // Determinar método efectivo y valor
      let (used_method, value, quality) = if allow_manual {
        if let Some(v) = self.manual_value_for(&smiles, prop, step4_input) {
          (ADMETSAMethod::Manual, v, "manual".to_string())
        } else if chosen == ADMETSAMethod::Manual {
          // Fallback a un método no-manual preferido si existe; si no, 0.0
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
      } else {
        // Manual no permitido: si el elegido es Manual, intentar fallback
        if chosen == ADMETSAMethod::Manual {
          if let Some(m2) =
            base_input.preferred_methods.iter().copied().find(|m| *m != ADMETSAMethod::Manual && m.can_generate(prop))
          {
            (m2, m2.calculate_mock_value(prop), "calculated".to_string())
          } else {
            // Último recurso, método Random4 cubre todas en nuestra simulación
            (ADMETSAMethod::Random4, ADMETSAMethod::Random4.calculate_mock_value(prop), "calculated".to_string())
          }
        } else {
          (chosen, chosen.calculate_mock_value(prop), "calculated".to_string())
        }
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
                                             "family_id": family_id.to_string(),
                                             "source": "ADMETSAInitialStep4",
                                             "method": format!("{:?}", used_method)
                                           }) };
      props.push(owned);
    }
    Ok(props)
  }

  pub fn execute_step(&self, ctx: &StepContext, input: Step4Input) -> Result<crate::step::StepInfo, WorkflowError> {
    // Validar dependencias: Step3 (moléculas iniciales) y Step2 (métodos)
    let step3_payload: Option<Step3Payload> = ctx.get_step_payload_by_name_typed("MoleculeInitialStep3")?;
    let step3_payload = step3_payload.ok_or_else(|| WorkflowError::Validation("Falta resultado de Step3".into()))?;
    let (allow_manual, base_input) = self.step2_allowed_manual(ctx)?;

    // Obtener familia desde Step1 si se requiere para metadata
    let step1_payload: Option<crate::flows::cadma_flow::steps::family_reference_step1::Step1Payload> =
      ctx.get_step_payload_by_name_typed("FamilyReferenceStep1")?;
    let family_id = step1_payload.map(|p| p.family_uuid).unwrap_or(Uuid::nil());

    // Cargar moléculas iniciales
    let mut saved_ids: Vec<String> = Vec::new();
    let mut generated_for: Vec<String> = Vec::new();
    for ik in step3_payload.generated_molecules.iter() {
      if let Some(mol) = ctx.domain_repo.get_molecule(ik)?.as_ref() {
        let props = self.compute_for_molecule(mol, &family_id, &base_input, &input, allow_manual)?;
        // Guardar propiedades en repo de dominio
        for p in props {
          let id = ctx.domain_repo.save_molecular_property(p)?;
          saved_ids.push(id.to_string());
        }
        generated_for.push(ik.clone());
      }
    }

    let payload = Step4Payload { generated_for: generated_for.clone(),
                                 saved_property_ids: saved_ids.clone(),
                                 calculated_properties: saved_ids.len(),
                                 step_result: "ok".into() };
    let metadata = Step4Metadata { status: "ok".into(), parameters: Step4Params { input }, domain_refs: generated_for };
    Ok(crate::step::StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }
}

impl_workflow_step!(ADMETSAInitialStep4,
                    Step4Payload,
                    Step4Metadata,
                    Step4Input,
                    |this_self, ctx, input| { this_self.execute_step(ctx, input) });
