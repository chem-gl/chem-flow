use super::admetsa_types::{ADMETSAMethod, ADMETSAProperty};
use chem_domain::{Molecule, OwnedMolecularProperty};
use uuid::Uuid;
/// Componente reusable para calcular y construir propiedades de moléculas.
#[derive(Debug, Clone)]
pub struct PropertyCalculator;
impl PropertyCalculator {
  /// Construye una propiedad calculada (mock) con metadatos comunes.
  pub fn build_calculated_property(&self,
                                   molecule: &Molecule,
                                   family_id: &Uuid,
                                   prop: ADMETSAProperty,
                                   method: ADMETSAMethod)
                                   -> OwnedMolecularProperty {
    let inchikey = molecule.inchikey().to_string();
    let v = method.calculate_mock_value(prop);
    let metadata = serde_json::json!({
      "method": format!("{:?}", method),
      "family_id": family_id.to_string(),
      "step": "ADMETSAPropertiesStep2"
    });
    OwnedMolecularProperty { id: Uuid::new_v4(),
                             molecule_inchikey: inchikey,
                             property_type: format!("{:?}", prop),
                             value: serde_json::json!(v),
                             quality: Some("calculated".to_string()),
                             preferred: true,
                             value_hash: format!("{:?}_{}", prop, v),
                             metadata }
  }
  /// Construye una propiedad manual con metadatos comunes.
  pub fn build_manual_property(&self,
                               molecule: &Molecule,
                               family_id: &Uuid,
                               prop: ADMETSAProperty,
                               value: f64)
                               -> OwnedMolecularProperty {
    let inchikey = molecule.inchikey().to_string();
    let metadata = serde_json::json!({
      "method": "manual",
      "family_id": family_id.to_string(),
      "step": "ADMETSAPropertiesStep2"
    });
    OwnedMolecularProperty { id: Uuid::new_v4(),
                             molecule_inchikey: inchikey,
                             property_type: format!("{:?}", prop),
                             value: serde_json::json!(value),
                             quality: Some("manual".to_string()),
                             preferred: true,
                             value_hash: format!("{:?}_{}", prop, value),
                             metadata }
  }
}
