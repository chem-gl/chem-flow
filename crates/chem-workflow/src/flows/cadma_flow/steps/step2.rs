use crate::errors::WorkflowError;
use crate::step::WorkflowStep;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Paso de ejemplo que crea una familia de moléculas.
pub struct Step2;
impl Step2 {
  /// Ejecutar el paso usando `StepContext` e `input` JSON.
  ///
  /// El `ctx` se usa para leer outputs previos (p.ej. `step1`) y `input`
  /// aporta parámetros dinámicos (p.ej. `multiplier`). Devuelve `StepInfo`
  /// listo para persistir. Los helpers `into_stepinfo` y `recover_from`
  /// sirven para serializar/rehidratar los DTOs tipados.
  pub fn execute(&self, ctx: &crate::step::StepContext, input: &JsonValue) -> Result<crate::step::StepInfo, WorkflowError> {
    use crate::flows::cadma_flow::steps::step1::Step1Payload;

    // Obtener valor tipado de step1 desde persistencia usando el contexto
    let prev: Option<Step1Payload> = ctx.get_typed_output("step1")?;
    let prev = match prev {
      Some(p) => p,
      None => return Err(WorkflowError::Validation("No se encontró payload previo de step1".to_string())),
    };

    // Leer multiplicador del input JSON (compatibilidad con enteros o floats)
    let multiplier = match input.get("multiplier") {
      Some(v) => {
        if v.is_i64() {
          v.as_i64().unwrap()
        } else if v.is_u64() {
          v.as_u64().unwrap() as i64
        } else if v.is_f64() {
          v.as_f64().unwrap() as i64
        } else {
          return Err(WorkflowError::Validation("Tipo de multiplicador inválido; se esperaba un número".to_string()));
        }
      }
      None => return Err(WorkflowError::Validation("Falta 'multiplier' en input".to_string())),
    };

    // Construir payload resultante
    let computed = prev.saved_value * multiplier;
    let payload = Step2Payload { generated_family: "from-step1".to_string(),
                                 step_result: format!("Computed {} * {} = {}", prev.saved_value, multiplier, computed),
                                 saved_value: computed };
    let metadata = Step2Metadata { status: "completed".to_string(),
                                   parameters: Step2Params { molecules: vec![] },
                                   domain_refs: vec![prev.generated_molecule.clone()] };

    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }

  /// Serializa el payload y metadata a `StepInfo` (helper interno para
  /// pruebas y rehidratación). Queda `pub(crate)` para uso dentro del crate.
  #[allow(dead_code)]
  pub(crate) fn into_stepinfo(payload: &Step2Payload,
                              metadata: &Step2Metadata)
                              -> Result<crate::step::StepInfo, WorkflowError> {
    Ok(crate::step::StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }

  /// Reconstruye el DTO tipado desde `StepInfo` (helper interno).
  pub fn recover_from(info: &crate::step::StepInfo) -> Result<Step2Payload, WorkflowError> {
    let p: Step2Payload = serde_json::from_value(info.payload.clone())?;
    Ok(p)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Payload {
  /// Identificador de la familia generada (por ejemplo un UUID string).
  pub generated_family: String,
  /// Mensaje/resultados del paso.
  pub step_result: String,
  /// Valor numérico calculado y guardado para pasos posteriores.
  pub saved_value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Params {
  /// Lista de SMILES o identificadores de las moléculas usadas.
  pub molecules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Metadata {
  /// Estado del paso (ej. "completed").
  pub status: String,
  /// Parámetros del paso.
  pub parameters: Step2Params,
  /// Referencias a objetos de dominio producidos.
  pub domain_refs: Vec<String>,
}

impl WorkflowStep for Step2 {
  fn name(&self) -> &str {
    "step2"
  }
  fn execute(&self, ctx: &crate::step::StepContext, input: &JsonValue) -> crate::step::StepResult {
    self.execute(ctx, input)
  }
}
