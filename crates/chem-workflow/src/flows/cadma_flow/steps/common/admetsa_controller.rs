use super::admetsa_types::{ADMETSAMethod, ADMETSAProperty, ManualValues, MethodPropertyMap, REQUIRED_PROPERTIES};
use crate::errors::WorkflowError;
use std::collections::{HashMap, HashSet};

/// Controlador de lógica de selección y validación ADMETSA reutilizable entre
/// pasos.
#[derive(Debug, Clone)]
pub struct ADMETSAController;

impl ADMETSAController {
  /// Valida que el conjunto de métodos (preferidos + mapeo) cubra las
  /// propiedades requeridas.
  pub fn validate_methods_cover(&self,
                                preferred_methods: &[ADMETSAMethod],
                                method_property_map: &Option<MethodPropertyMap>)
                                -> Result<(), WorkflowError> {
    let mut covered = HashSet::<ADMETSAProperty>::new();

    if let Some(map) = method_property_map {
      for (&prop, &method) in map.iter() {
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
      let ok = preferred_methods.iter().any(|&m| m.can_generate(prop));
      if !ok {
        return Err(WorkflowError::Validation(format!("Ningún método preferido puede generar {:?}", prop)));
      }
      covered.insert(prop);
    }

    Ok(())
  }

  /// Elige el método para una propiedad: mapa explícito > preferidos > Manual
  pub fn choose_method(&self,
                       prop: ADMETSAProperty,
                       preferred_methods: &[ADMETSAMethod],
                       method_property_map: &Option<MethodPropertyMap>)
                       -> ADMETSAMethod {
    if let Some(map) = method_property_map {
      if let Some(&m) = map.get(&prop) {
        return m;
      }
    }
    preferred_methods.iter().copied().find(|&m| m.can_generate(prop)).unwrap_or(ADMETSAMethod::Manual)
  }

  /// Busca valor manual por SMILES y propiedad.
  pub fn manual_value_for(&self, smiles: &str, prop: ADMETSAProperty, manual_values: &Option<ManualValues>) -> Option<f64> {
    let prop_key = format!("{:?}", prop);
    manual_values.as_ref().and_then(|mv| mv.get(smiles)).and_then(|pv| pv.get(&prop_key).copied())
  }

  /// Helper: agrupa entradas generadas por tipo de propiedad.
  pub fn group_by_property<'a, T>(&self,
                                  items: &'a [T],
                                  key_selector: impl Fn(&'a T) -> &'a String)
                                  -> HashMap<String, Vec<&'a T>> {
    let mut map: HashMap<String, Vec<&T>> = HashMap::new();
    for item in items {
      map.entry(key_selector(item).clone()).or_default().push(item);
    }
    map
  }
}
