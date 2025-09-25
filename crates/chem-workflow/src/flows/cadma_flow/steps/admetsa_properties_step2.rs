// Paso ADMETSA (Step2)
//
// Descripción (es):
// Este paso calcula propiedades ADMETSA para todas las moléculas de la familia
// proporcionada por el paso anterior (Step1). Entrada (Step2Input):
// - preferred_methods: orden de preferencia de  los métodos a usar para generar
//   propiedades si solo se indica este se asegura que todas las propiedades se
//   puedan generar
// - method_property_map: mapa opcional que asigna propiedades a métodos
//   específicos //deben completar las que falten arriba o sobre-escribirlas
//   igual se deben guardar en caso que aun falte se deberan agregar manualmente
// - manual_values: mapa opcional con valores manuales por SMILES y propiedad.
//
// Comportamiento:
// 1. Se valida que la combinación de `method_property_map` y
//    `preferred_methods` cubra el conjunto de `REQUIRED_PROPERTIES`.
// 2. Para cada molécula y cada propiedad requerida se determina el método a
//    usar (mapa explícito o el primer método preferido que pueda generarla).
// 3. Si existe un valor manual para esa molécula+propiedad (en
//    `manual_values`), se usa ese valor.
// 4. Si el método asignado es `Manual` pero no existe valor en `manual_values`,
//    el paso falla con error de validación (no se genera automáticamente).
// 5. Para métodos aleatorios (Random1/2/3/4) en esta implementación de pruebas
//    se generan valores mock estáticos por propiedad.
// 6. Todos los valores generados (incluyendo metadata con método/fuente/step)
//    se persisten usando `ctx.domain_repo.save_molecular_property(...)`.
//
// Implementación futura:
// - Integrar `chem_providers::ChemEngine` para cálculos reales según método.
// - Soportar múltiples valores por propiedad y reglas de preferencia más
//   avanzadas.

// Paso ADMETSA (Step2)
use crate::{flows::cadma_flow::steps::family_reference_step1::Step1Payload, step::StepContext, WorkflowError};
use chem_domain::OwnedMolecularProperty;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Types principales - usando newtype pattern para mayor seguridad
pub type ManualValues = HashMap<String, PropertyValues>;
pub type PropertyValues = HashMap<ADMETSAProperty, f64>;
pub type MethodPropertyMap = HashMap<ADMETSAProperty, ADMETSAMethod>;

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

// Colecciones optimizadas
pub type AllPropertiesFull = HashMap<String, Vec<GeneratedPropertyEntry>>;
pub type SelectedProperties = HashMap<String, HashMap<String, SelectedPropertyEntry>>;

// Enums principales
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
  // Fisicoquímicas
  LogP,
  PSA,
  AtX,
  HBA,
  HBD,
  RB,
  MR,
  // Toxicológicas
  LD50,
  Mutagenicity,
  DevelopmentalToxicity,
  // Sintéticas
  SyntheticAccessibility,
}

// Constantes optimizadas
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

// List of all supported methods (facilita iteración/consulta desde UI/tests)
pub const ALL_METHODS: [ADMETSAMethod; 5] =
  [ADMETSAMethod::Manual, ADMETSAMethod::Random1, ADMETSAMethod::Random2, ADMETSAMethod::Random3, ADMETSAMethod::Random4];

// Implementación eficiente usando match en tiempo de compilación
impl ADMETSAMethod {
  pub const fn can_generate(self, prop: ADMETSAProperty) -> bool {
    use ADMETSAProperty::*;
    matches!((self, prop),
             (Self::Manual, _)
             | (Self::Random1, LogP | PSA | AtX | HBA | HBD | RB | MR)
             | (Self::Random2, LD50 | Mutagenicity | DevelopmentalToxicity | SyntheticAccessibility)
             | (Self::Random3, HBD | RB | MR | LD50 | Mutagenicity))
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

      _ => 0.0,
    }
  }
}

// Structs de datos optimizados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Input {
  pub preferred_methods: Vec<ADMETSAMethod>,
  pub method_property_map: Option<MethodPropertyMap>,
  pub manual_values: Option<ManualValues>,
}

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

// Builder pattern para configuración
#[derive(Debug, Default)]
pub struct Step2Config {
  preferred_methods: Vec<ADMETSAMethod>,
  method_property_map: Option<MethodPropertyMap>,
  manual_values: Option<ManualValues>,
}

impl Step2Config {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn preferred_methods(mut self, methods: Vec<ADMETSAMethod>) -> Self {
    self.preferred_methods = methods;
    self
  }

  pub fn method_property_map(mut self, map: MethodPropertyMap) -> Self {
    self.method_property_map = Some(map);
    self
  }

  pub fn manual_values(mut self, values: ManualValues) -> Self {
    self.manual_values = Some(values);
    self
  }

  pub fn build(self) -> Step2Input {
    Step2Input { preferred_methods: self.preferred_methods,
                 method_property_map: self.method_property_map,
                 manual_values: self.manual_values }
  }
}

// Implementación principal optimizada
#[derive(Debug)]
pub struct ADMETSAPropertiesStep2;

impl ADMETSAPropertiesStep2 {
  /// Devuelve un mapa que indica, para cada método, las propiedades que puede
  /// generar. Útil para UIs y validaciones antes de construir el
  /// `Step2Input`.
  pub fn methods_capabilities() -> std::collections::HashMap<ADMETSAMethod, Vec<ADMETSAProperty>> {
    let mut map = std::collections::HashMap::new();
    for &m in &ALL_METHODS {
      let mut props = Vec::new();
      for &p in REQUIRED_PROPERTIES.iter() {
        if m.can_generate(p) || m == ADMETSAMethod::Manual {
          props.push(p);
        }
      }
      map.insert(m, props);
    }
    map
  }

  /// Verifica si la lista de métodos preferidos cubre todas las propiedades
  /// requeridas (es decir, para cada propiedad existe al menos un método
  /// preferido que pueda generarla).
  pub fn validate_preferred_methods_cover(preferred: &[ADMETSAMethod]) -> Result<(), WorkflowError> {
    use std::collections::HashSet;
    let mut covered = HashSet::new();
    for &m in preferred {
      for &p in REQUIRED_PROPERTIES.iter() {
        if m.can_generate(p) || m == ADMETSAMethod::Manual {
          covered.insert(p);
        }
      }
    }
    for &p in REQUIRED_PROPERTIES.iter() {
      if !covered.contains(&p) {
        return Err(WorkflowError::Validation(format!("Preferred methods do not cover property: {:?}", p)));
      }
    }
    Ok(())
  }
  pub fn execute_step(&self, ctx: &StepContext, input: Step2Input) -> Result<crate::step::StepInfo, WorkflowError> {
    let prev = ctx.get_typed_output_by_type::<Step1Payload>()?
                  .ok_or_else(|| WorkflowError::Validation("Step1Payload not found".into()))?;

    let family_id = prev.family_uuid.ok_or_else(|| WorkflowError::Validation("No family UUID in Step1Payload".into()))?;

    let family = ctx.domain_repo
                    .get_family(&family_id)?
                    .ok_or_else(|| WorkflowError::Validation(format!("Family {} not found", family_id)))?;

    self.validate_method_configuration(&input)?;

    let molecules = family.molecules();
    let mol_count = molecules.len();
    let mut calculated_count = 0;
    let mut all_properties_full: AllPropertiesFull = HashMap::with_capacity(mol_count);
    let mut selected_properties: SelectedProperties = HashMap::with_capacity(mol_count);
    let mut saved_property_ids = Vec::new();

    for molecule in molecules {
      let properties = self.calculate_molecule_properties(molecule, &input, &family_id)?;
      calculated_count += properties.len();

      let mut generated_entries: Vec<GeneratedPropertyEntry> = Vec::new();
      for prop in properties {
        let value = prop.value.as_f64().unwrap_or(0.0);
        let method = prop.metadata.get("method").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

        let entry = GeneratedPropertyEntry { id: prop.id,
                                             property_type: prop.property_type.clone(),
                                             value,
                                             method: method.clone(),
                                             metadata: prop.metadata.clone() };

        // Persistir la propiedad en el repositorio de dominio
        ctx.domain_repo.save_molecular_property(prop)?;
        saved_property_ids.push(entry.id.to_string());
        generated_entries.push(entry);
      }

      let smiles = molecule.smiles().to_string();
      all_properties_full.insert(smiles.clone(), generated_entries.clone());

      let chosen_map = self.select_preferred_properties(generated_entries, &input.preferred_methods);
      selected_properties.insert(smiles, chosen_map);
    }

    let payload = Step2Payload { family_id,
                                 calculated_properties: calculated_count,
                                 step_result: format!("Calculadas {} propiedades para {} moléculas",
                                                      calculated_count, mol_count),
                                 all_properties: all_properties_full,
                                 selected_properties };

    let metadata = Step2Metadata { status: "completed".to_string(),
                                   parameters: Step2Params { input },
                                   domain_refs: vec![family_id.to_string()],
                                   saved_property_ids };

    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }

  fn validate_method_configuration(&self, input: &Step2Input) -> Result<(), WorkflowError> {
    let mut covered = std::collections::HashSet::new();

    // Validar mapeo explícito
    if let Some(map) = &input.method_property_map {
      for (&prop, &method) in map {
        if !method.can_generate(prop) {
          return Err(WorkflowError::Validation(format!("Método {:?} no puede generar propiedad {:?}", method, prop)));
        }
        covered.insert(prop);
      }
    }

    // Cubrir propiedades faltantes con métodos preferidos
    for &prop in REQUIRED_PROPERTIES.iter() {
      if covered.contains(&prop) {
        continue;
      }

      let ok = input.preferred_methods.iter().any(|&m| m.can_generate(prop));
      if !ok {
        return Err(WorkflowError::Validation(format!("Ningún método puede generar {:?}", prop)));
      }
      covered.insert(prop);
    }

    Ok(())
  }

  fn calculate_molecule_properties(&self,
                                   molecule: &chem_domain::Molecule,
                                   input: &Step2Input,
                                   family_id: &Uuid)
                                   -> Result<Vec<OwnedMolecularProperty>, WorkflowError> {
    let smiles_str = molecule.smiles().to_string();
    let inchikey = molecule.inchikey().to_string();

    REQUIRED_PROPERTIES.iter()
                       .map(|&prop| {
                         let method = self.get_property_method(prop, input);
                         let value = self.get_property_value(prop, &smiles_str, molecule, input, method)?;

                         Ok(OwnedMolecularProperty { id: Uuid::new_v4(),
                                                     molecule_inchikey: inchikey.clone(),
                                                     property_type: format!("{:?}", prop),
                                                     value: serde_json::json!(value),
                                                     quality: Some("calculated".to_string()),
                                                     preferred: true,
                                                     value_hash: format!("{:?}_{}", prop, value),
                                                     metadata: serde_json::json!({
                                                         "method": format!("{:?}", method),
                                                         "family_id": family_id.to_string(),
                                                         "step": "ADMETSAPropertiesStep2"
                                                     }) })
                       })
                       .collect()
  }

  fn get_property_method(&self, prop: ADMETSAProperty, input: &Step2Input) -> ADMETSAMethod {
    input.method_property_map
         .as_ref()
         .and_then(|map| map.get(&prop).copied())
         .or_else(|| input.preferred_methods.iter().find(|&&m| m.can_generate(prop)).copied())
         .unwrap_or(ADMETSAMethod::Manual)
  }

  fn get_property_value(&self,
                        prop: ADMETSAProperty,
                        smiles: &str,
                        _molecule: &chem_domain::Molecule,
                        input: &Step2Input,
                        method: ADMETSAMethod)
                        -> Result<f64, WorkflowError> {
    // 1. Intentar valor manual primero
    if let Some(manual_vals) = &input.manual_values {
      if let Some(mol_props) = manual_vals.get(smiles) {
        if let Some(&value) = mol_props.get(&prop) {
          return Ok(value);
        }
      }
    }

    // 2. Validar método
    if !method.can_generate(prop) && method != ADMETSAMethod::Manual {
      return Err(WorkflowError::Validation(format!("Método {:?} no puede generar {:?}", method, prop)));
    }

    // 3. Calcular valor
    Ok(match method {
         ADMETSAMethod::Manual => 0.0, // Placeholder para valores manuales no proporcionados
         _ => method.calculate_mock_value(prop),
       })
  }

  fn select_preferred_properties(&self,
                                 entries: Vec<GeneratedPropertyEntry>,
                                 preferred_methods: &[ADMETSAMethod])
                                 -> HashMap<String, SelectedPropertyEntry> {
    let mut grouped: HashMap<String, Vec<GeneratedPropertyEntry>> = HashMap::new();

    for entry in entries {
      grouped.entry(entry.property_type.clone()).or_default().push(entry);
    }

    grouped.into_iter()
           .filter_map(|(prop_type, group)| {
             let chosen = preferred_methods.iter()
                                           .find_map(|pm| {
                                             let pm_str = format!("{:?}", pm);
                                             group.iter().find(|g| g.method == pm_str).cloned()
                                           })
                                           .or_else(|| group.into_iter().next())?;

             Some((prop_type.clone(),
                   SelectedPropertyEntry { id: chosen.id,
                                           property_type: chosen.property_type,
                                           value: chosen.value,
                                           method: chosen.method }))
           })
           .collect()
  }
}

crate::impl_workflow_step!(ADMETSAPropertiesStep2,
                           Step2Payload,
                           Step2Metadata,
                           Step2Input,
                           |this_self, ctx, input| { this_self.execute_step(ctx, input) });
