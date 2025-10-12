/// Claves y helpers estandarizados para eventos y estados de pasos.
pub const KEY_PREFIX_STEP_STATE: &str = "step_state:";

/// Construye la clave estandarizada para el estado de un paso.
pub fn key_for_step_state(step_name: &str) -> String {
  format!("{}{}", KEY_PREFIX_STEP_STATE, step_name)
}
