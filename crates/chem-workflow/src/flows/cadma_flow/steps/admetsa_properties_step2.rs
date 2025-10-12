// admetsa_properties_step2.rs
//! Paso 2: calcular propiedades ADMETSA para todas las moléculas de la familia
//! creada/seleccionada en Step1.
//! - Soporta "method_property_map" y "preferred_methods".
//! - Los valores manuales se pueden suministrar por SMILES.
//! - Guarda cada propiedad mediante los ports del dominio como
//!   OwnedMolecularProperty.

use crate::errors::WorkflowError;
use crate::flows::cadma_flow::steps::common::{
  ADMETSAController, ADMETSAMethod, ADMETSAProperty, ManualValues, MethodPropertyMap, PropertyCalculator,
  REQUIRED_PROPERTIES,
};
use crate::flows::cadma_flow::steps::family_reference_step1::Step1Payload;
use crate::step::StepContext;
use chem_domain::{Molecule, OwnedMolecularProperty};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Tipos ahora vienen de common

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Input {
  pub preferred_methods: Vec<ADMETSAMethod>,
  pub method_property_map: Option<MethodPropertyMap>,
  pub manual_values: Option<ManualValues>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPropertyEntry {
  pub id: Uuid,
  pub property_type: String,
  pub value: f64,
  pub method: String,
  pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedPropertyEntry {
  pub id: Uuid,
  pub property_type: String,
  pub value: f64,
  pub method: String,
}

pub type AllPropertiesFull = HashMap<String, Vec<GeneratedPropertyEntry>>; // SMILES -> entries
pub type SelectedProperties = HashMap<String, HashMap<String, SelectedPropertyEntry>>; // SMILES -> prop -> chosen entry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Payload {
  pub family_id: Uuid,
  pub calculated_properties: usize,
  pub step_result: String,
  pub all_properties: AllPropertiesFull,
  pub selected_properties: SelectedProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Metadata {
  pub status: String,
  pub parameters: Step2Params,
  pub domain_refs: Vec<String>,
  pub saved_property_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Params {
  pub input: Step2Input,
}

#[derive(Debug, Default, Clone)]
pub struct ADMETSAPropertiesStep2;

impl ADMETSAPropertiesStep2 {
  /// Validación principal: el mapeo + métodos preferidos deben cubrir las
  /// propiedades requeridas.
  fn validate_methods_cover(&self, input: &Step2Input) -> Result<(), WorkflowError> {
    ADMETSAController.validate_methods_cover(&input.preferred_methods, &input.method_property_map)
  }

  /// Obtiene el método a usar para una propiedad (mapa explícito > preferencia
  /// > Manual por default)
  fn choose_method(&self, prop: ADMETSAProperty, input: &Step2Input) -> ADMETSAMethod {
    ADMETSAController.choose_method(prop, &input.preferred_methods, &input.method_property_map)
  }

  /// Intenta obtener valor manual si existe; la clave interna en ManualValues
  /// se hace con `format!("{:?}", prop)`.
  fn manual_value_for(&self, smiles: &str, prop: ADMETSAProperty, input: &Step2Input) -> Option<f64> {
    ADMETSAController.manual_value_for(smiles, prop, &input.manual_values)
  }

  /// Calcula (mock) los OwnedMolecularProperty para una molécula.
  fn compute_properties_for_molecule(&self,
                                     molecule: &Molecule,
                                     family_id: &Uuid,
                                     input: &Step2Input)
                                     -> Result<Vec<OwnedMolecularProperty>, WorkflowError> {
    let smiles = molecule.smiles().to_string();
    let mut props = Vec::with_capacity(REQUIRED_PROPERTIES.len());

    for &prop in &REQUIRED_PROPERTIES {
      let method = self.choose_method(prop, input);

      // Prioridad: manual_values override
      if let Some(v) = self.manual_value_for(&smiles, prop, input) {
        let calc = PropertyCalculator;
        props.push(calc.build_manual_property(molecule, family_id, prop, v));
        continue;
      }

      // Si método es Manual y no hay valor, intentamos fallback a un método
      // preferido capaz de generar la propiedad. Si no hay ninguno, error.
      if method == ADMETSAMethod::Manual {
        if let Some(m_pref) =
          input.preferred_methods.iter().copied().find(|&m| m != ADMETSAMethod::Manual && m.can_generate(prop))
        {
          let calc = PropertyCalculator;
          props.push(calc.build_calculated_property(molecule, family_id, prop, m_pref));
          continue;
        } else {
          return Err(WorkflowError::Validation(format!("Método Manual asignado para {:?} pero no existe valor manual \
                                                        para SMILES {}",
                                                       prop, smiles)));
        }
      }

      // Asegurar que el método pueda generar la propiedad
      if !method.can_generate(prop) {
        return Err(WorkflowError::Validation(format!("Método {:?} no puede generar la propiedad {:?}", method, prop)));
      }

      let calc = PropertyCalculator;
      props.push(calc.build_calculated_property(molecule, family_id, prop, method));
    }

    Ok(props)
  }

  /// Selecciona la mejor entrada por propiedad según `preferred_methods` (si no
  /// hay match, toma la primera).
  fn select_preferred(&self,
                      generated: &[GeneratedPropertyEntry],
                      preferred_methods: &[ADMETSAMethod])
                      -> HashMap<String, SelectedPropertyEntry> {
    let by_prop = ADMETSAController.group_by_property(generated, |g| &g.property_type);

    let pref_strs: Vec<String> = preferred_methods.iter().map(|m| format!("{:?}", m)).collect();
    let mut chosen = HashMap::with_capacity(by_prop.len());

    for (prop_type, group) in by_prop {
      if let Some(best) = group.iter().find(|&&g| pref_strs.contains(&g.method)).cloned() {
        chosen.insert(prop_type.clone(),
                      SelectedPropertyEntry { id: best.id,
                                              property_type: best.property_type.clone(),
                                              value: best.value,
                                              method: best.method.clone() });
      } else if let Some(first) = group.first() {
        chosen.insert(prop_type.clone(),
                      SelectedPropertyEntry { id: first.id,
                                              property_type: first.property_type.clone(),
                                              value: first.value,
                                              method: first.method.clone() });
      }
    }

    chosen
  }

  /// Ejecuta el step: lee Step1Payload, recorre moléculas, calcula y persiste
  /// propiedades.
  pub fn execute_step(&self, ctx: &StepContext, input: Step2Input) -> Result<crate::step::StepInfo, WorkflowError> {
    // Obtener payload de step1 (familia)
    let prev = ctx.get_typed_output_by_type::<Step1Payload>()?
                  .ok_or_else(|| WorkflowError::Validation("Step1Payload not found".into()))?;
    let family_id = prev.family_uuid;

    // Obtener familia
    let family = ctx.domain_repo
                    .get_family(&family_id)?
                    .ok_or_else(|| WorkflowError::Validation(format!("Family {} not found", family_id)))?;

    // Validar configuración de métodos
    self.validate_methods_cover(&input)?;

    let molecules: Vec<&Molecule> = family.molecules().iter().collect();
    let mol_count = molecules.len();

    let mut all_properties: AllPropertiesFull = HashMap::with_capacity(mol_count);
    let mut selected_properties: SelectedProperties = HashMap::with_capacity(mol_count);
    let mut saved_ids: Vec<String> = Vec::with_capacity(mol_count * REQUIRED_PROPERTIES.len());
    let mut domain_refs: Vec<String> = vec![family_id.to_string()];

    for mol in molecules {
      let props = self.compute_properties_for_molecule(mol, &family_id, &input)?;
      // convertir a GeneratedPropertyEntry y persistir cada OwnedMolecularProperty en
      // domain repo
      let mut generated_entries: Vec<GeneratedPropertyEntry> = Vec::with_capacity(props.len());
      for p in props.into_iter() {
        // persistir (clonar p porque save_molecular_property consume
        // OwnedMolecularProperty)
        ctx.domain_repo.save_molecular_property(p.clone())?;
        saved_ids.push(p.id.to_string());

        // construir entry para el retorno
        let v = p.value.as_f64().unwrap_or(0.0);
        let method = p.metadata.get("method").and_then(|m| m.as_str()).unwrap_or("unknown").to_string();
        generated_entries.push(GeneratedPropertyEntry { id: p.id,
                                                        property_type: p.property_type,
                                                        value: v,
                                                        method: method.clone(),
                                                        metadata: p.metadata });
      }

      let smiles = mol.smiles().to_string();
      let inchikey = mol.inchikey().to_string();
      domain_refs.push(inchikey.clone());
      let chosen = self.select_preferred(&generated_entries, &input.preferred_methods);
      all_properties.insert(smiles.clone(), generated_entries);
      selected_properties.insert(smiles, chosen);
    }

    let calc_count = saved_ids.len();
    let payload = Step2Payload { family_id,
                                 calculated_properties: calc_count,
                                 step_result: format!("Calculadas {} propiedades para {} moléculas",
                                                      calc_count, mol_count),
                                 all_properties,
                                 selected_properties };

    let metadata = Step2Metadata { status: "completed".to_string(),
                                   parameters: Step2Params { input },
                                   domain_refs,
                                   saved_property_ids: saved_ids };

    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }
}

crate::impl_workflow_step!(ADMETSAPropertiesStep2,
                           Step2Payload,
                           Step2Metadata,
                           Step2Input,
                           |this_self, ctx, input| { this_self.execute_step(ctx, input) });
