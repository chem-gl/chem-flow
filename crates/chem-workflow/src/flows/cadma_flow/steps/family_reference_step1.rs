// family_reference_step1.rs
//! Paso 1: seleccionar / fusionar / crear una familia de moléculas.
//! Objetivo: recibir familias existentes y/o moléculas explícitas, evitar
//! duplicados por InChIKey, y o bien seleccionar una familia existente (caso
//! simple) o crear/fusionar una nueva familia persistida en el
//! DomainRepository.

use crate::errors::WorkflowError;
use crate::step::{StepContext, StepInfo};
use chem_domain::{Molecule, MoleculeFamily};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Default, Clone)]
pub struct FamilyReferenceStep1;

/// Entrada del paso: IDs de familias, moléculas explícitas y opcional
/// nombre/desc de nueva familia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Input {
  pub families: Option<Vec<Uuid>>,
  pub molecules: Option<Vec<Molecule>>,
  pub new_family_name: Option<String>,
  pub new_family_description: Option<String>,
}

/// Payload que se persiste en flow_data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Payload {
  pub family_uuid: Uuid,
  pub step_result: String,
  pub molecules_count: usize,
}

/// Metadatos legibles para auditoría / UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Metadata {
  pub status: String,
  pub parameters: Step1Params,
  pub domain_refs: Vec<String>, // inchikeys + family id
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Params {
  pub families: Option<Vec<Uuid>>,
  pub molecules_count: usize,
  pub new_family_name: Option<String>,
}

impl FamilyReferenceStep1 {
  /// Caso rápido: si el usuario sólo pasó UNA familia y no pidió crear una
  /// nueva, seleccionamos esa familia y devolvemos su payload/metadata sin
  /// modificar el dominio.
  fn try_select_single_existing(&self, ctx: &StepContext, input: &Step1Input) -> Result<Option<StepInfo>, WorkflowError> {
    if input.new_family_name.is_some() {
      return Ok(None);
    }
    if let Some(fids) = &input.families {
      if fids.len() == 1 && input.molecules.is_none() {
        let fid = fids[0];
        let family = ctx.domain_repo
                        .get_family(&fid)?
                        .ok_or_else(|| WorkflowError::Validation(format!("Familia {} no encontrada", fid)))?;
        let molecules_count = family.molecules().len();
        let payload = Step1Payload { family_uuid: fid,
                                     step_result: format!("Familia existente seleccionada: {}", fid),
                                     molecules_count };
        let domain_refs: Vec<String> =
          family.molecules().iter().map(|m| m.inchikey().to_string()).chain(std::iter::once(fid.to_string())).collect();
        let metadata = Step1Metadata { status: "completed".to_string(),
                                       parameters: Step1Params { families: input.families.clone(),
                                                                 molecules_count,
                                                                 new_family_name: None },
                                       domain_refs };
        return Ok(Some(StepInfo { payload: serde_json::to_value(&payload)?, metadata: serde_json::to_value(&metadata)? }));
      }
    }
    Ok(None)
  }
  fn collect_unique_molecules(&self,
                              ctx: &StepContext,
                              families: &Option<Vec<Uuid>>,
                              explicit: &Option<Vec<Molecule>>)
                              -> Result<Vec<Molecule>, WorkflowError> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();

    if let Some(explicit_mols) = explicit {
      for m in explicit_mols {
        let key = m.inchikey().to_string();
        if seen.insert(key) {
          out.push(m.clone());
        }
      }
    }

    if let Some(fids) = families {
      for fid in fids {
        let family = ctx.domain_repo
                        .get_family(fid)?
                        .ok_or_else(|| WorkflowError::Validation(format!("Familia {} no encontrada", fid)))?;
        // iterar por copia de las moléculas del family para evitar borrows largos
        for m in family.molecules().iter().cloned() {
          let key = m.inchikey().to_string();
          if seen.insert(key) {
            out.push(m);
          }
        }
      }
    }

    Ok(out)
  }

  /// Construye y persiste una nueva familia usando `DomainRepository`.
  fn create_and_save_family(&self,
                            ctx: &StepContext,
                            molecules: Vec<Molecule>,
                            name: Option<String>,
                            description: Option<String>)
                            -> Result<(Uuid, usize, Vec<String>), WorkflowError> {
    let provenance = json!({
        "created_by": "family_reference_step1",
        "name": name.clone().unwrap_or_default(),
        "timestamp": Utc::now().to_rfc3339()
    });
    let mut family = MoleculeFamily::new(molecules.into_iter(), provenance)?;
    if let Some(n) = name {
      family = family.with_name(n);
    }
    if let Some(d) = description {
      family = family.with_description(d);
    }
    let saved_id = ctx.domain_repo.save_family(family)?;
    let stored_family = ctx.domain_repo.get_family(&saved_id)?.ok_or_else(|| {
                                                                 WorkflowError::Persistence(format!("Familia {} \
                                                                                                     persistida pero no \
                                                                                                     encontrada",
                                                                                                    saved_id))
                                                               })?;
    let domain_refs: Vec<String> = stored_family.molecules()
                                                .iter()
                                                .map(|m| m.inchikey().to_string())
                                                .chain(std::iter::once(saved_id.to_string()))
                                                .collect();
    let total = stored_family.molecules().len();
    Ok((saved_id, total, domain_refs))
  }

  /// Implementación principal del execute: combina los helpers previos.
  fn execute_step_impl(&self, ctx: &StepContext, input: Step1Input) -> Result<StepInfo, WorkflowError> {
    if let Some(info) = self.try_select_single_existing(ctx, &input)? {
      return Ok(info);
    }

    let explicit_count = input.molecules.as_ref().map_or(0, |v| v.len());
    let collected = self.collect_unique_molecules(ctx, &input.families, &input.molecules)?;

    if collected.is_empty() {
      return Err(WorkflowError::Validation("No hay moléculas (ni en familias ni explícitas) para crear/fusionar una \
                                            familia"
                                                    .into()));
    }

    let (saved_id, total_molecules, domain_refs) = self.create_and_save_family(ctx,
                                                                               collected,
                                                                               input.new_family_name.clone(),
                                                                               input.new_family_description.clone())?;

    let payload = Step1Payload { family_uuid: saved_id,
                                 step_result: format!("Familia creada/fusionada: {} ({} moléculas)",
                                                      saved_id, total_molecules),
                                 molecules_count: total_molecules };

    let metadata = Step1Metadata { status: "completed".to_string(),
                                   parameters: Step1Params { families: input.families,
                                                             molecules_count: explicit_count,
                                                             new_family_name: input.new_family_name },
                                   domain_refs };

    Ok(StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }
}

crate::impl_workflow_step!(FamilyReferenceStep1,
                           Step1Payload,
                           Step1Metadata,
                           Step1Input,
                           |this_self, ctx, input| { this_self.execute_step_impl(ctx, input) });
