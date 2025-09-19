use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::error::Error;
use uuid::Uuid;

/// Trait generico para motores de flujo quimicos.
///
/// Este trait define la interfaz minima que debe exponer un motor de
/// workflow quimico. El motor es responsable de ejecutar pasos, manejar
/// snapshots y crear ramas. No asume la implementacion concreta del
/// dominio (molculas/familias), por eso opera con `serde_json::Value`
/// para la entrada y salida y expone el `WorkflowConfig` para indicar
/// como debe persistirse la informacion.
pub trait ChemicalFlowEngine: Send + Sync {
    /// Identificador del flow asociado a esta instancia.
    fn id(&self) -> Uuid;

    /// Ejecuta el siguiente paso del flujo y retorna el estado/result
    /// como `JsonValue` cuando se completa correctamente.
    ///
    /// Los errores se devuelven como `Box<dyn Error>` para permitir que la
    /// implementacion concreta propague errores de persistencia o del dominio.
    fn execute_next(&mut self) -> Result<JsonValue, Box<dyn Error>>;

    /// Aplica un snapshot serializado para rehidratar el estado del motor.
    ///
    /// El snapshot debe ser un `JsonValue` producido por `snapshot()` y
    /// representar el estado completo necesario para reanudar la ejecucion.
    fn apply_snapshot(&mut self, snapshot: &JsonValue) -> Result<(), Box<dyn Error>>;

    /// Extrae el estado serializado listo para almacenarse como snapshot.
    ///
    /// Debe producir un `JsonValue` autocontenido que `apply_snapshot`
    /// pueda volver a aplicar.
    fn snapshot(&self) -> Result<JsonValue, Box<dyn Error>>;

    /// Crea una rama a partir de un cursor/version dada.
    ///
    /// `parent_cursor` indica el punto de corte desde el cual se bifurca.
    /// `name` y `status` son metadatos opcionales para la nueva rama.
    fn create_branch(&self,
                     parent_cursor: i64,
                     name: Option<String>,
                     status: Option<String>)
                     -> Result<Uuid, Box<dyn Error>>;

    /// Devuelve la configuracion activa del workflow.
    fn config(&self) -> WorkflowConfig;
}

/// Configuracion minima expuesta por el motor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub persistence_mode: PersistenceMode,
    pub snapshot_policy: SnapshotPolicy,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        WorkflowConfig { persistence_mode: PersistenceMode::Embedded,
                         snapshot_policy: SnapshotPolicy::Never }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PersistenceMode {
    Embedded,
    SeparateTables,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SnapshotPolicy {
    Never,
    Every(i64),
}
