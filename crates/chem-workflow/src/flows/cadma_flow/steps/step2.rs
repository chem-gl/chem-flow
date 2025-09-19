use crate::errors::WorkflowError;
use crate::step::WorkflowStep;
use chem_domain::{Molecule, MoleculeFamily};
use serde_json::Value as JsonValue;

/// Paso de ejemplo que crea una familia de moleculas.
pub struct Step2;

impl WorkflowStep for Step2 {
    fn name(&self) -> &str {
        "step2"
    }

    fn execute(&self, _input: &JsonValue) -> Result<crate::step::StepOutput, WorkflowError> {
        // Simular creacion de una familia a partir de moleculas
        let m1 = Molecule::from_smiles("CCO").map_err(WorkflowError::Domain)?;
        let m2 = Molecule::from_smiles("CCN").map_err(WorkflowError::Domain)?;
        let family = MoleculeFamily::new(vec![m1, m2], JsonValue::Object(serde_json::json!({"source": "step2"}).as_object().unwrap().clone())).map_err(WorkflowError::Domain)?;
        let family_id = family.id();

        // Payload: datos generados
        let payload = JsonValue::Object(serde_json::json!({
                                            "generated_family": family_id.to_string(),
                                            "step_result": "Family created successfully"
                                        }).as_object()
                                          .unwrap()
                                          .clone());

        // Metadata: parametros, referencias, status
        let metadata = JsonValue::Object(serde_json::json!({
                                             "status": "completed",
                                             "parameters": {"molecules": ["CCO", "CCN"]},
                                             "domain_refs": [family_id.to_string()]
                                         }).as_object()
                                           .unwrap()
                                           .clone());

        // Produced domain refs
        let produced_domain_refs = vec![family_id];

        Ok(crate::step::StepOutput { produced_domain_refs,
                                     payload,
                                     metadata })
    }
}
