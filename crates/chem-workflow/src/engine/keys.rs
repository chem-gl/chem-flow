//! Módulo de claves para el almacenamiento de estados de pasos.
pub const STEP_STATE_PREFIX: &str = "step_state:";

pub fn step_state_key(step_name: &str) -> String {
  format!("{}{}", STEP_STATE_PREFIX, step_name)
}
