use crate::errors::WorkflowError;
use crate::step::StepContext;
use chem_domain::DomainRepository;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
/// Resultado de ejecutar un paso. Contiene datos para persistir en `FlowData` o
/// tablas de dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInfo {
  pub payload: JsonValue,
  pub metadata: JsonValue,
}
pub type StepResult = Result<StepInfo, WorkflowError>;
/// Trait principal para pasos de workflow con tipos fuertemente tipados
pub trait WorkflowStep: Send + Sync {
  type Payload: Serialize + DeserializeOwned + Send + Sync + 'static;
  type Metadata: Serialize + DeserializeOwned + Send + Sync + 'static;
  type Input: DeserializeOwned + Send + Sync + 'static;
  fn name(&self) -> &str;
  fn execute_typed(&self, ctx: &StepContext, input: Self::Input) -> StepResult;
  /// Inicialización opcional donde el paso puede recibir el repositorio de
  /// dominio (por ejemplo para mantenerlo en un campo interno). Por defecto
  /// es no-op; pasos que necesiten el repo pueden sobreescribir este método.
  fn init(&mut self, _domain_repo: Arc<dyn DomainRepository>) {}
  // Métodos de utilidad para conversión tipada
  fn into_stepinfo(payload: &Self::Payload, metadata: &Self::Metadata) -> Result<StepInfo, WorkflowError> {
    Ok(StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }
  fn recover_payload(info: &StepInfo) -> Result<Self::Payload, WorkflowError> {
    serde_json::from_value(info.payload.clone()).map_err(Into::into)
  }
  // Implementación por defecto para compatibilidad con JSON
  fn execute(&self, ctx: &StepContext, input: &JsonValue) -> StepResult {
    let parsed = serde_json::from_value(input.clone())?;
    self.execute_typed(ctx, parsed)
  }
}
/// Trait object-safe para dispatch dinámico en runtime
pub trait WorkflowStepDyn: Send + Sync {
  fn name(&self) -> &str;
  fn execute(&self, ctx: &StepContext, input: &JsonValue) -> StepResult;
  /// Inicialización que permite inyectar el `DomainRepository` cuando el
  /// engine crea/obtiene el paso. Por defecto es no-op; implementaciones
  /// concretas pueden almacenar el repo si lo necesitan.
  fn init(&mut self, _domain_repo: Arc<dyn DomainRepository>);
}
// Implementación automática del trait dinámico para todos los WorkflowStep
impl<T> WorkflowStepDyn for T where T: WorkflowStep
{
  fn name(&self) -> &str {
    WorkflowStep::name(self)
  }
  fn execute(&self, ctx: &StepContext, input: &JsonValue) -> StepResult {
    WorkflowStep::execute(self, ctx, input)
  }
  fn init(&mut self, domain_repo: Arc<dyn DomainRepository>) {
    // Delegate to the typed trait default/override so concrete steps can
    // implement `init` on the `WorkflowStep` trait and receive the repo.
    WorkflowStep::init(self, domain_repo)
  }
}
// Macro helper para reducir boilerplate al definir pasos simples
#[macro_export]
macro_rules! impl_workflow_step {
  // Variante básica que infiere el nombre del tipo
  ($step_ty:ident, $payload:ty, $metadata:ty, $input:ty) => {
    impl $crate::step::WorkflowStep for $step_ty {
      type Payload = $payload;
      type Metadata = $metadata;
      type Input = $input;
      fn name(&self) -> &str {
        stringify!($step_ty)
      }
      fn execute_typed(&self, ctx: &$crate::step::StepContext, input: Self::Input) -> $crate::step::StepResult {
        self.run_typed(ctx, input)
      }
    }
  };
  // Variante con implementación inline del método run_typed
  // Variant that accepts an explicit `self` identifier plus ctx and input
  ($step_ty:ident, $payload:ty, $metadata:ty, $input:ty, |$self_ident:ident, $ctx_ident:ident, $input_ident:ident| $body:block) => {
    impl $step_ty {
      pub fn run_typed(&self, $ctx_ident: &$crate::step::StepContext, $input_ident: $input) -> $crate::step::StepResult {
        let $self_ident = self;
        $body
      }
    }
    $crate::impl_workflow_step!($step_ty, $payload, $metadata, $input);
  };
  // Variante con implementación inline del método run_typed (ctx, input only)
  ($step_ty:ident, $payload:ty, $metadata:ty, $input:ty, |$ctx_ident:ident, $input_ident:ident| $body:block) => {
    impl $step_ty {
      pub fn run_typed(&self, $ctx_ident: &$crate::step::StepContext, $input_ident: $input) -> $crate::step::StepResult {
        $body
      }
    }
    $crate::impl_workflow_step!($step_ty, $payload, $metadata, $input);
  };
}
