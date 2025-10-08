/// Claves y helpers para la convención de eventos/keys del flujo.
pub const STEP_STATE_PREFIX: &str = "step_state:";

/// Construye la key de estado de un paso siguiendo la convención establecida.
pub fn step_state_key(step_name: &str) -> String {
  format!("{}{}", STEP_STATE_PREFIX, step_name)
}
