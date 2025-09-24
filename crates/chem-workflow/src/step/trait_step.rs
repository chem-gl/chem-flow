use crate::errors::WorkflowError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Resultado de ejecutar un paso. Puede contener datos que deben ser
/// persistidos ya sea embebidos en `FlowData` o en tablas separadas del
/// repositorio de dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInfo {
  /// Payload principal del paso: DTO serializado que representa el resultado.
  pub payload: JsonValue,
  /// Metadatos operativos del paso (estado, parámetros, referencias a IDs).
  pub metadata: JsonValue,
}

pub type StepResult = Result<StepInfo, WorkflowError>;

/// Trait que define la interfaz comun para todos los pasos de un workflow.
pub trait WorkflowStep: Send + Sync {
  /// Nombre o identificador del paso
  fn name(&self) -> &str;
  /// Guarda estado interno del paso si aplica (opcional).
  fn save_state(&self) -> Result<(), WorkflowError> {
    Ok(())
  }
  /// Carga estado interno del paso si aplica (opcional).
  fn load_state(&self) -> Result<(), WorkflowError> {
    Ok(())
  }
  /// Lista de pasos previos cuyos datos son requeridos por este paso.
  /// Por defecto no requiere nada.
  /// Lista de nombres de pasos previos cuyos resultados son requeridos.
  /// Por defecto no requiere nada.
  fn required_previous_steps(&self) -> Vec<String> {
    Vec::new()
  }

  /// Ejecuta el paso usando el contexto (`StepContext`) y la entrada JSON.
  ///
  /// Esta es la única API pública que el engine invoca. El `ctx` permite
  /// acceder a repositorios y a helpers tipados (por ejemplo
  /// `StepContext::get_typed_output`) y `input` contiene parámetros
  /// dinámicos en JSON. Debe devolver un `StepInfo` listo para persistir.
  fn execute(&self, _ctx: &crate::step::StepContext, _input: &JsonValue) -> StepResult {
    // Implementación por defecto que obliga a cada paso a proporcionar
    // su propia lógica de ejecución si necesita comportamiento concreto.
    Err(WorkflowError::Validation("execute no implementado para este paso".to_string()))
  }

  // - `StepInfo.payload` es el lugar donde debe colocarse el DTO principal
  //   generado por el paso (por ejemplo, `Step2Payload`). Este `payload` será
  //   guardado por el engine usando una `FlowData` con key
  //   `step_state:{step_name}`. Ver `CadmaFlow::persist_step_result`.
  // - `StepInfo.metadata` contiene metadatos operativos del paso (status,
  //   parámetros, referencias a objetos de dominio como IDs). Esto se guarda
  //   junto al payload en el mismo registro `FlowData.metadata`.
  // - Para rehidratar, el engine o los tests deben leer `FlowData` (por ejemplo
  //   con `flow_repo.read_data`) y reconstruir un `StepInfo`; luego cada paso
  //   puede llamar a su `recover_from(&StepInfo)` para obtener el DTO tipado.
  //   Ejemplo: `let typed = Step2::recover_from(&info)?;`.
  // - Desde la perspectiva del engine existe una única forma de invocar un paso:
  //   `execute(&ctx, &input)`. Los pasos pueden mantener helpers privados para
  //   construir DTOs tipados y serializarlos internamente.
}
