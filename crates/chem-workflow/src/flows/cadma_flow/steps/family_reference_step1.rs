use chem_domain::{Molecule, MoleculeFamily};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::step::StepContext;

/// Paso para fusionar familias existentes o crear una nueva familia
pub struct FamilyReferenceStep1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Step1Input {
  pub families: Option<Vec<Uuid>>,
  pub molecules: Option<Vec<Molecule>>,
  pub new_family_name: Option<String>,
  pub new_family_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Payload {
  pub family_uuid: Option<Uuid>,
  pub step_result: String,
  pub molecules_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Params {
  pub families: Option<Vec<Uuid>>,
  pub molecules_count: usize,
  pub new_family_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Metadata {
  pub status: String,
  pub parameters: Step1Params,
  pub domain_refs: Vec<String>,
}

impl FamilyReferenceStep1 {
  /// Maneja el caso simple de selección de familia existente
  fn existing_family(&self,
                     ctx: &crate::step::StepContext,
                     family_id: Uuid,
                     input: &Step1Input)
                     -> Result<crate::step::StepInfo, crate::errors::WorkflowError> {
    let family =
      ctx.domain_repo
         .get_family(&family_id)?
         .ok_or_else(|| crate::errors::WorkflowError::Validation(format!("Familia {} no encontrada", family_id)))?;

    let payload = Step1Payload { family_uuid: Some(family_id),
                                 step_result: format!("Familia existente seleccionada: {}", family_id),
                                 molecules_count: family.molecules().len() };

    let metadata = Step1Metadata { status: "completed".to_string(),
                                   parameters: Step1Params { families: input.families.clone(),
                                                             molecules_count: 0,
                                                             new_family_name: input.new_family_name.clone() },
                                   domain_refs: vec![family_id.to_string()] };

    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }

  /// Recolecta todas las moléculas de familias y lista explícita
  fn collect_molecules(&self,
                       ctx: &crate::step::StepContext,
                       families: &Option<Vec<Uuid>>,
                       explicit_molecules: &Option<Vec<Molecule>>)
                       -> Result<Vec<Molecule>, crate::errors::WorkflowError> {
    let mut collected = Vec::new();

    // Agregar moléculas explícitas
    if let Some(mols) = explicit_molecules {
      collected.extend_from_slice(mols);
    }

    // Agregar moléculas de familias existentes
    if let Some(family_ids) = families {
      for family_id in family_ids {
        let family = ctx.domain_repo.get_family(family_id)?.ok_or_else(|| {
                                                              crate::errors::WorkflowError::Validation(
                        format!("Familia {} no encontrada", family_id)
                    )
                                                            })?;

        collected.extend(family.molecules().iter().cloned());
      }
    }

    Ok(collected)
  }

  /// Crea una nueva familia a partir de las moléculas recolectadas
  fn create_new_family(&self,
                       molecules: Vec<Molecule>,
                       name: Option<String>,
                       description: Option<String>)
                       -> Result<MoleculeFamily, crate::errors::WorkflowError> {
    let provenance = json!({
        "created_by": "family_reference_step1",
        "name": name.clone().unwrap_or_default(),
        "timestamp": Utc::now().to_rfc3339()
    });

    let mut family = MoleculeFamily::new(molecules.into_iter(), provenance)?;

    if let Some(name) = name {
      family = family.with_name(name)?;
    }

    if let Some(desc) = description {
      family = family.with_description(desc)?;
    }

    Ok(family)
  }

  /// Ejecuta el paso principal
  fn execute_step(&self,
                  ctx: &StepContext,
                  input: Step1Input)
                  -> Result<crate::step::StepInfo, crate::errors::WorkflowError> {
    let explicit_mol_count = input.molecules.as_ref().map_or(0, |v| v.len());

    // Caso 1: Solo una familia existente y no se agregaron moléculas explícitas
    // Si además no se pidió un nuevo nombre, no creamos una nueva familia
    // y usamos la existente.
    if input.molecules.is_none() {
      if let Some(family_ids) = &input.families {
        if family_ids.len() == 1 && input.new_family_name.is_none() {
          return self.existing_family(ctx, family_ids[0], &input);
        }
      }
    }

    let molecules = self.collect_molecules(ctx, &input.families, &input.molecules)?;

    if molecules.is_empty() {
      return Err(crate::errors::WorkflowError::Validation("No hay moléculas para crear/fusionar una familia".into()));
    }

    let new_family = self.create_new_family(molecules, input.new_family_name.clone(), input.new_family_description.clone())?;

    // Número total de moléculas en la nueva familia (después de fusionar y
    // deduplicar). Usamos esto en la salida para evitar confusiones con el
    // conteo de moléculas explícitas pasado por el usuario.
    let total_molecules = new_family.molecules().len();

    let saved_id = ctx.domain_repo.save_family(new_family)?;

    // Construir domain_refs
    let mut domain_refs = vec![saved_id.to_string()];
    if let Some(mols) = &input.molecules {
      domain_refs.extend(mols.iter().map(|m| m.inchikey().to_string()));
    }

    let payload = Step1Payload { family_uuid: Some(saved_id),
                                 step_result: format!("Familia creada/fusionada: {} ({} moléculas)",
                                                      saved_id, total_molecules),
                                 molecules_count: total_molecules };

    let metadata = Step1Metadata { status: "completed".to_string(),
                                   parameters: Step1Params { families: input.families.clone(),
                                                             molecules_count: explicit_mol_count,
                                                             new_family_name: input.new_family_name },
                                   domain_refs };

    Ok(crate::step::StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? })
  }
}

crate::impl_workflow_step!(FamilyReferenceStep1,
                           Step1Payload,
                           Step1Metadata,
                           Step1Input,
                           |this_self, ctx, input| { this_self.execute_step(ctx, input) });
