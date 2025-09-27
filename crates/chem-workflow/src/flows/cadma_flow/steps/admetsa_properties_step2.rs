// admetsa_properties_step2.rs
//! Paso 2: calcular propiedades ADMETSA para todas las moléculas de la familia
//! creada/seleccionada en Step1.
//! - Soporta "method_property_map" y "preferred_methods".
//! - Los valores manuales se pueden suministrar por SMILES.
//! - Guarda cada propiedad en domain_repo como OwnedMolecularProperty.

use crate::errors::WorkflowError;
use crate::flows::cadma_flow::steps::family_reference_step1::Step1Payload;
use crate::step::StepContext;
use chem_domain::{Molecule, OwnedMolecularProperty};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Tipos auxiliares:
/// ManualValues: SMILES -> (prop_name_string -> value)
pub type PropertyValues = HashMap<String, f64>;
pub type ManualValues = HashMap<String, PropertyValues>;
pub type MethodPropertyMap = HashMap<ADMETSAProperty, ADMETSAMethod>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ADMETSAMethod {
  Manual,
  Random1,
  Random2,
  Random3,
  Random4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ADMETSAProperty {
  LogP,
  PSA,
  AtX,
  HBA,
  HBD,
  RB,
  MR,
  LD50,
  Mutagenicity,
  DevelopmentalToxicity,
  SyntheticAccessibility,
}

pub const REQUIRED_PROPERTIES: [ADMETSAProperty; 11] = [ADMETSAProperty::LogP,
                                                        ADMETSAProperty::PSA,
                                                        ADMETSAProperty::AtX,
                                                        ADMETSAProperty::HBA,
                                                        ADMETSAProperty::HBD,
                                                        ADMETSAProperty::RB,
                                                        ADMETSAProperty::MR,
                                                        ADMETSAProperty::LD50,
                                                        ADMETSAProperty::Mutagenicity,
                                                        ADMETSAProperty::DevelopmentalToxicity,
                                                        ADMETSAProperty::SyntheticAccessibility];

pub const ALL_METHODS: [ADMETSAMethod; 5] =
  [ADMETSAMethod::Manual, ADMETSAMethod::Random1, ADMETSAMethod::Random2, ADMETSAMethod::Random3, ADMETSAMethod::Random4];

impl ADMETSAMethod {
  pub const fn can_generate(self, prop: ADMETSAProperty) -> bool {
    use ADMETSAProperty::*;
    matches!((self, prop),
             (Self::Manual, _)
             | (Self::Random1, LogP | PSA | AtX | HBA | HBD | RB | MR)
             | (Self::Random2, LD50 | Mutagenicity | DevelopmentalToxicity | SyntheticAccessibility)
             | (Self::Random3, HBD | RB | MR | LD50 | Mutagenicity)
             | (Self::Random4, _))
  }

  pub const fn calculate_mock_value(self, prop: ADMETSAProperty) -> f64 {
    match (self, prop) {
      (Self::Random1, ADMETSAProperty::LogP) => 2.5,
      (Self::Random1, ADMETSAProperty::PSA) => 45.0,
      (Self::Random1, ADMETSAProperty::AtX) => 24.0,
      (Self::Random1, ADMETSAProperty::HBA) => 3.0,
      (Self::Random1, ADMETSAProperty::HBD) => 1.0,
      (Self::Random1, ADMETSAProperty::RB) => 5.0,
      (Self::Random1, ADMETSAProperty::MR) => 60.0,

      (Self::Random2, ADMETSAProperty::LD50) => 350.0,
      (Self::Random2, ADMETSAProperty::Mutagenicity) => 0.0,
      (Self::Random2, ADMETSAProperty::DevelopmentalToxicity) => 0.0,
      (Self::Random2, ADMETSAProperty::SyntheticAccessibility) => 3.2,

      (Self::Random3, ADMETSAProperty::HBD) => 2.0,
      (Self::Random3, ADMETSAProperty::RB) => 3.0,
      (Self::Random3, ADMETSAProperty::MR) => 72.0,
      (Self::Random3, ADMETSAProperty::LD50) => 250.0,
      (Self::Random3, ADMETSAProperty::Mutagenicity) => 1.0,

      (Self::Random4, ADMETSAProperty::LogP) => 3.1,
      (Self::Random4, ADMETSAProperty::PSA) => 50.0,
      (Self::Random4, ADMETSAProperty::AtX) => 25.0,
      (Self::Random4, ADMETSAProperty::HBA) => 4.0,
      (Self::Random4, ADMETSAProperty::HBD) => 1.5,
      (Self::Random4, ADMETSAProperty::RB) => 4.0,
      (Self::Random4, ADMETSAProperty::MR) => 65.0,
      (Self::Random4, ADMETSAProperty::LD50) => 300.0,
      (Self::Random4, ADMETSAProperty::Mutagenicity) => 0.5,
      (Self::Random4, ADMETSAProperty::DevelopmentalToxicity) => 0.2,
      (Self::Random4, ADMETSAProperty::SyntheticAccessibility) => 2.8,

      _ => 0.0,
    }
  }
}

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
    let mut covered = HashSet::<ADMETSAProperty>::new();

    if let Some(map) = &input.method_property_map {
      for (&prop, &method) in map {
        if !method.can_generate(prop) {
          return Err(WorkflowError::Validation(format!("Método {:?} no puede generar la propiedad {:?}", method, prop)));
        }
        covered.insert(prop);
      }
    }

    for &prop in &REQUIRED_PROPERTIES {
      if covered.contains(&prop) {
        continue;
      }
      let ok = input.preferred_methods.iter().any(|&m| m.can_generate(prop));
      if !ok {
        return Err(WorkflowError::Validation(format!("Ningún método preferido puede generar {:?}", prop)));
      }
      covered.insert(prop);
    }

    Ok(())
  }

  /// Obtiene el método a usar para una propiedad (mapa explícito > preferencia
  /// > Manual por default)
  fn choose_method(&self, prop: ADMETSAProperty, input: &Step2Input) -> ADMETSAMethod {
    if let Some(map) = &input.method_property_map {
      if let Some(&m) = map.get(&prop) {
        return m;
      }
    }
    input.preferred_methods.iter().copied().find(|&m| m.can_generate(prop)).unwrap_or(ADMETSAMethod::Manual)
  }

  /// Intenta obtener valor manual si existe; la clave interna en ManualValues
  /// se hace con `format!("{:?}", prop)`.
  fn manual_value_for(&self, smiles: &str, prop: ADMETSAProperty, input: &Step2Input) -> Option<f64> {
    let prop_key = format!("{:?}", prop);
    input.manual_values.as_ref().and_then(|mv| mv.get(smiles)).and_then(|pv| pv.get(&prop_key).copied())
  }

  /// Calcula (mock) los OwnedMolecularProperty para una molécula.
  fn compute_properties_for_molecule(&self,
                                     molecule: &Molecule,
                                     family_id: &Uuid,
                                     input: &Step2Input)
                                     -> Result<Vec<OwnedMolecularProperty>, WorkflowError> {
    let smiles = molecule.smiles().to_string();
    let inchikey = molecule.inchikey().to_string();
    let mut props = Vec::with_capacity(REQUIRED_PROPERTIES.len());

    for &prop in &REQUIRED_PROPERTIES {
      let method = self.choose_method(prop, input);

      // Prioridad: manual_values override
      if let Some(v) = self.manual_value_for(&smiles, prop, input) {
        let metadata = serde_json::json!({
            "method": "manual",
            "family_id": family_id.to_string(),
            "step": "ADMETSAPropertiesStep2"
        });
        props.push(OwnedMolecularProperty { id: Uuid::new_v4(),
                                            molecule_inchikey: inchikey.clone(),
                                            property_type: format!("{:?}", prop),
                                            value: serde_json::json!(v),
                                            quality: Some("manual".to_string()),
                                            preferred: true,
                                            value_hash: format!("{:?}_{}", prop, v),
                                            metadata });
        continue;
      }

      // Si método es Manual y no hay valor, intentamos fallback a un método
      // preferido capaz de generar la propiedad. Si no hay ninguno, error.
      if method == ADMETSAMethod::Manual {
        if let Some(m_pref) =
          input.preferred_methods.iter().copied().find(|&m| m != ADMETSAMethod::Manual && m.can_generate(prop))
        {
          let v = m_pref.calculate_mock_value(prop);
          let metadata = serde_json::json!({
              "method": format!("{:?}", m_pref),
              "family_id": family_id.to_string(),
              "step": "ADMETSAPropertiesStep2"
          });
          props.push(OwnedMolecularProperty { id: Uuid::new_v4(),
                                              molecule_inchikey: inchikey.clone(),
                                              property_type: format!("{:?}", prop),
                                              value: serde_json::json!(v),
                                              quality: Some("calculated".to_string()),
                                              preferred: true,
                                              value_hash: format!("{:?}_{}", prop, v),
                                              metadata });
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

      let v = method.calculate_mock_value(prop);
      let metadata = serde_json::json!({
          "method": format!("{:?}", method),
          "family_id": family_id.to_string(),
          "step": "ADMETSAPropertiesStep2"
      });

      props.push(OwnedMolecularProperty { id: Uuid::new_v4(),
                                          molecule_inchikey: inchikey.clone(),
                                          property_type: format!("{:?}", prop),
                                          value: serde_json::json!(v),
                                          quality: Some("calculated".to_string()),
                                          preferred: true,
                                          value_hash: format!("{:?}_{}", prop, v),
                                          metadata });
    }

    Ok(props)
  }

  /// Selecciona la mejor entrada por propiedad según `preferred_methods` (si no
  /// hay match, toma la primera).
  fn select_preferred(&self,
                      generated: &[GeneratedPropertyEntry],
                      preferred_methods: &[ADMETSAMethod])
                      -> HashMap<String, SelectedPropertyEntry> {
    let mut by_prop: HashMap<String, Vec<&GeneratedPropertyEntry>> = HashMap::new();
    for e in generated {
      by_prop.entry(e.property_type.clone()).or_default().push(e);
    }

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
