use crate::errors::WorkflowError;
use crate::step::WorkflowStep;
use chem_domain::Molecule;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Paso de ejemplo que crea una molecula desde SMILES.
pub struct Step1;

impl Step1 {
  /// Implementación única de ejecución: recibe `StepContext` (para poder
  /// leer outputs previos y acceder a repositorios) y `input` JSON con
  /// parámetros. Devuelve `StepInfo` listo para persistir.
  pub fn execute(&self,
                 _ctx: &crate::step::StepContext,
                 _input: &JsonValue)
                 -> Result<crate::step::StepInfo, WorkflowError> {
    // Simular creacion de una molecula desde SMILES
    let molecule = Molecule::from_smiles("CCO").map_err(WorkflowError::Domain)?;
    let inchikey = molecule.inchikey().to_string();

    // Añadimos un valor numérico que puede ser usado por pasos posteriores
    let payload = Step1Payload { generated_molecule: inchikey.clone(),
                                 step_result: "Molecule created successfully".to_string(),
                                 saved_value: 42 };
    let metadata = Step1Metadata { status: "completed".to_string(),
                                   parameters: Step1Params { smiles: "CCO".to_string() },
                                   domain_refs: vec![inchikey.clone()] };
    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }

  /// Serializa el payload y metadata a `StepInfo` (helper interno para
  /// rehidratación/tests). Queda `pub(crate)` para uso dentro del crate.
  #[allow(dead_code)]
  pub(crate) fn into_stepinfo(payload: &Step1Payload,
                              metadata: &Step1Metadata)
                              -> Result<crate::step::StepInfo, WorkflowError> {
    Ok(crate::step::StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }

  /// Reconstruye el DTO tipado desde `StepInfo` (helper interno).
  pub fn recover_from(info: &crate::step::StepInfo) -> Result<Step1Payload, WorkflowError> {
    let p: Step1Payload = serde_json::from_value(info.payload.clone())?;
    Ok(p)
  }
}

// COMENTARIOS EXPLICITOS (ES):
// - ¿Dónde se guarda el payload y metadata? El `StepInfo` que retorna
//   `execute`, `execute_typed` (a traves de `into_stepinfo`) contiene `payload`
//   (el DTO serializado) y `metadata`. Para persistir, el engine llama a
//   `persist_step_result("step1", info, ...)` que empaqueta `StepInfo.payload`
//   en `FlowData.payload` y `StepInfo.metadata` en `FlowData.metadata` con key
//   `step_state:step1`.
// - ¿Como se rehidrata? Para rehidratar se lee el `FlowData` correspondiente
//   (por ejemplo con `CadmaFlow::get_last_step_payload` o
//   `flow_repo.read_data`) y luego `Step1::recover_from(&step_info)`
//   reconstruye el DTO tipado desde `StepInfo.payload`.
// - ¿Qué funciones regresan el valor del paso? `execute_typed()` devuelve los
//   DTOs tipados (payload, metadata). `execute()` y `execute_with_context()`
//   devuelven `StepInfo` listo para persistir.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Payload {
  /// Identificador/representación generada de la molécula (ej. InChIKey).
  pub generated_molecule: String,
  /// Mensaje o resultado textual del paso.
  pub step_result: String,
  /// Valor numérico guardado que puede ser usado por pasos posteriores.
  pub saved_value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Params {
  /// SMILES usado para generar la molécula.
  pub smiles: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Metadata {
  /// Estado del paso (por ejemplo "completed").
  pub status: String,
  /// Parámetros usados por el paso (tipado).
  pub parameters: Step1Params,
  /// Referencias a objetos de dominio (por ejemplo InChIKey o IDs).
  pub domain_refs: Vec<String>,
}

impl WorkflowStep for Step1 {
  fn name(&self) -> &str {
    "step1"
  }
  fn execute(&self, ctx: &crate::step::StepContext, input: &JsonValue) -> crate::step::StepResult {
    // delegate to the single inherent execute implementation
    self.execute(ctx, input)
  }
}
