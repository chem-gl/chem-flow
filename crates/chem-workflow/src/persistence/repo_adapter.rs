use crate::engine::PersistenceMode;
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Adapter que segun el modo de persistencia decide como persistir outputs.
///
/// Esta estructura actua como punto central para decidir si el resultado
/// de un paso debe almacenarse embebido en `FlowData` (Embedded) o si la
/// persistencia debe realizarse en tablas separadas (SeparateTables).
pub struct RepoAdapter;

impl RepoAdapter {
    pub fn persist_step_output(_flow_id: &Uuid,
                               _cursor: i64,
                               _mode: PersistenceMode,
                               _output: &JsonValue)
                               -> Result<(), Box<dyn std::error::Error>> {
        match _mode {
            PersistenceMode::Embedded => {
                // In embedded mode the step output is stored inside FlowData
                // payload/metadata. The FlowRepository caller is expected to
                // persist the FlowData. Adapter does nothing here.
                Ok(())
            }
            PersistenceMode::SeparateTables => {
                // In SeparateTables mode we would persist domain objects using
                // DomainRepository. This adapter does not have access to the
                // domain repo; return an error signalling higher-level code
                // should handle persistence.
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "SeparateTables persistence must be handled by the flow engine with access to DomainRepository",
                )))
            }
        }
    }
}
