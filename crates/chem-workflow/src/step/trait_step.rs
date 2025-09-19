use crate::errors::WorkflowError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Resultado de ejecutar un paso. Puede contener datos que deben ser
/// persistidos ya sea embebidos en `FlowData` o en tablas separadas del
/// repositorio de dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub produced_domain_refs: Vec<Uuid>,
    pub payload: JsonValue,
    pub metadata: JsonValue,
}

pub type StepResult = Result<StepOutput, WorkflowError>;

/// Trait que representa un paso del flujo.
pub trait WorkflowStep: Send + Sync {
    /// Nombre o identificador del paso
    fn name(&self) -> &str;

    /// Validacion previa a la ejecucion. Puede usar el `input` para validar
    /// precondiciones y retornar `WorkflowError::Validation` si no se cumplen.
    fn validate(&self, _input: &JsonValue) -> Result<(), WorkflowError> {
        Ok(())
    }

    /// Ejecuta la logica del paso y devuelve el resultado
    fn execute(&self, _input: &JsonValue) -> StepResult;

    /// Indica si este paso genera objetos de dominio que deben persistirse
    /// mediante `DomainRepository` cuando el `WorkflowConfig` indique
    /// `PersistenceMode::SeparateTables`.
    fn requires_domain_persistence(&self) -> bool {
        false
    }
}
