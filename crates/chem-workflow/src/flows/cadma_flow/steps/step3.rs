use crate::errors::WorkflowError;
use crate::step::WorkflowStep;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Paso de ejemplo que combina resultados previos y produce un resumen.
pub struct Step3;

impl Step3 {
  /// Ejecutar el paso 3 usando `StepContext` e `input` JSON.
  ///
  /// Intent: leer el output tipado de `step2` desde el contexto y
  /// producir un resumen. Si el contexto no contiene el DTO, se intenta
  /// extraer `step2_output` desde `input` (compatibilidad hacia atrás).
  pub fn execute(&self, ctx: &crate::step::StepContext, input: &JsonValue) -> Result<crate::step::StepInfo, WorkflowError> {
    // Intentar leer desde contexto
    let mut maybe_step2: Option<crate::flows::cadma_flow::steps::step2::Step2Payload> = ctx.get_typed_output("step2")?;
    // Fallback: parsear de input bajo la clave `step2_output`
    if maybe_step2.is_none() {
      if let Some(map) = input.as_object() {
        maybe_step2 = map.get("step2_output").and_then(|v| serde_json::from_value(v.clone()).ok());
      }
    }

    let summary = maybe_step2.as_ref().map(|p| p.saved_value).unwrap_or(0);
    let payload = Step3Payload { summary_score: summary + 1, step_result: "Summary computed".to_string() };
    let metadata = Step3Metadata { status: "completed".to_string(), parameters: Step3Params {}, domain_refs: vec![] };
    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }

  #[allow(dead_code)]
  pub(crate) fn into_stepinfo(payload: &Step3Payload,
                              metadata: &Step3Metadata)
                              -> Result<crate::step::StepInfo, WorkflowError> {
    Ok(crate::step::StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }

  pub fn recover_from(info: &crate::step::StepInfo) -> Result<Step3Payload, WorkflowError> {
    let p: Step3Payload = serde_json::from_value(info.payload.clone())?;
    Ok(p)
  }
}

// COMENTARIOS EXPLICITOS (ES):
// - Guardado: `Step3::into_stepinfo` serializa `Step3Payload` en
//   `StepInfo.payload` y `Step3Metadata` en `StepInfo.metadata`. Cuando el
//   engine persiste esto con `persist_step_result("step3", info, ...)`, se
//   escriben en `FlowData.payload` y `FlowData.metadata` con key
//   `step_state:step3`.
// - Rehidratación: leer `FlowData` (por ejemplo con
//   `CadmaFlow::get_last_step_payload`) y luego usar `Step3::recover_from` para
//   obtener `Step3Payload` tipado.
// - Valores retornados: `execute_typed()` devuelve `(Step3Payload,
//   Step3Metadata)`; `execute()` y `execute_with_context()` retornan `StepInfo`
//   listo para persistir.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Payload {
  /// Puntuación o resumen computado por el paso.
  pub summary_score: i64,
  /// Mensaje/resultados del paso.
  pub step_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Params {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step3Metadata {
  /// Estado del paso.
  pub status: String,
  /// Parámetros del paso (vacío en este ejemplo).
  pub parameters: Step3Params,
  /// Referencias a objetos de dominio usadas o producidas.
  pub domain_refs: Vec<String>,
}

impl WorkflowStep for Step3 {
  fn name(&self) -> &str {
    "step3"
  }
  fn required_previous_steps(&self) -> Vec<String> {
    vec!["step2".to_string()]
  }
  fn execute(&self, ctx: &crate::step::StepContext, input: &JsonValue) -> crate::step::StepResult {
    self.execute(ctx, input)
  }
}
