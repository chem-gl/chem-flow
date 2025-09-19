use crate::errors::WorkflowError;
use crate::step::WorkflowStep;
use chem_domain::Molecule;
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Paso de ejemplo que crea una molecula desde SMILES.
pub struct Step1;

impl WorkflowStep for Step1 {
    fn name(&self) -> &str {
        "step1"
    }

    fn execute(&self, _input: &JsonValue) -> Result<crate::step::StepOutput, WorkflowError> {
        // Simular creacion de una molecula desde SMILES
        let molecule = Molecule::from_smiles("CCO").map_err(WorkflowError::Domain)?;
        let inchikey = molecule.inchikey().to_string();

        // Payload: datos generados (resultado del paso)
        let payload = JsonValue::Object(serde_json::json!({
                                            "generated_molecule": inchikey.clone(),
                                            "step_result": "Molecule created successfully"
                                        }).as_object()
                                          .unwrap()
                                          .clone());

        // Metadata: parametros, referencias, status
        let metadata = JsonValue::Object(serde_json::json!({
                                             "status": "completed",
                                             "parameters": {"smiles": "CCO"},
                                             "domain_refs": [inchikey.clone()]
                                         }).as_object()
                                           .unwrap()
                                           .clone());

        // Produced domain refs: IDs de objetos de dominio creados (placeholder)
        let produced_domain_refs = vec![Uuid::new_v4()];

        Ok(crate::step::StepOutput { produced_domain_refs,
                                     payload,
                                     metadata })
    }
}
