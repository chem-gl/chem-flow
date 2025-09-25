use crate::flows::cadma_flow::steps::step2::Step2Payload;
use crate::step::StepInfo;
use serde::{Deserialize, Serialize};
// ...existing code...

/// Paso de ejemplo que combina resultados previos y produce un resumen.
pub struct Step3;

// COMENTARIOS EXPLICITOS (ES):
// - Guardado: `Step3::into_stepinfo` serializa `Step3Payload` en
//   `StepInfo.payload` y `Step3Metadata` en `StepInfo.metadata`. Cuando el
//   engine persiste esto con `persist_step_result("step3", info, ...)`, se
//   escriben en `FlowData.payload` y `FlowData.metadata` con key
//   `step_state:step3`.
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

// The WorkflowStep impl is generated below via the `impl_workflow_step!` macro.

// Use the helper macro to generate the same impl/run_typed for Step3.
crate::impl_workflow_step!(Step3, Step3Payload, Step3Metadata, serde_json::Value, |ctx, _input| {
  // Prefer to find by type; fall back to explicit key or input-provided value.
  let maybe_step2 = ctx.get_typed_output_by_type::<Step2Payload>()?;

  let summary = maybe_step2.as_ref().map(|p| p.saved_value).unwrap_or(0);
  let payload = Step3Payload { summary_score: summary + 1, step_result: "Resumen calculado".to_string() };
  let metadata = Step3Metadata { status: "completed".to_string(), parameters: Step3Params {}, domain_refs: vec![] };
  Ok(StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
});
