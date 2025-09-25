use crate::flows::cadma_flow::steps::family_reference_step1::Step1Payload;
use serde::{Deserialize, Serialize};
/// Paso de ejemplo que crea una familia de moléculas.
pub struct Step2;
#[derive(Debug, Serialize, Deserialize)]
pub struct Step2Input {
  pub multiplier: i64,
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
// Generate the WorkflowStep impl using the helper macro. This enforces
// the associated types and delegates `execute_typed` to `run_typed`.
crate::impl_workflow_step!(Step2, Step2Payload, Step2Metadata, Step2Input, |ctx, _input| {
  // Try to fetch by type first (no need to pass step name explicitly).
  let prev_opt = ctx.get_typed_output_by_type::<Step1Payload>()?;
  let prev = prev_opt.ok_or_else(|| crate::errors::WorkflowError::Validation("Step1Payload not found in context".into()))?;
  // Imprimimos el id de la familia (si existe) y pasamos el número de
  // moléculas (desde Step1Payload.molecules_count) a Step3 via saved_value.
  let family_opt = prev.family_uuid;
  if let Some(fid) = family_opt {
    println!("🔔 Step2: familia seleccionada -> {}", fid);
  } else {
    println!("🔔 Step2: no se proporcionó familia, se creará/usar una nueva");
  }

  let molecules_count = prev.molecules_count as i64;

  let payload = Step2Payload { generated_family: family_opt.map(|u| u.to_string())
                                                           .unwrap_or_else(|| "generated-family".to_string()),
                               step_result: format!("Familia: {:?}, moléculas: {}", family_opt, molecules_count),
                               saved_value: molecules_count };

  let mut domain_refs = Vec::new();
  if let Some(fid) = family_opt {
    domain_refs.push(fid.to_string());
  }
  let metadata =
    Step2Metadata { status: "completed".to_string(), parameters: Step2Params { molecules: vec![] }, domain_refs };
  Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
});
