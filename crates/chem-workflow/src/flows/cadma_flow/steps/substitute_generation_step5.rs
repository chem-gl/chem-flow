use crate::errors::WorkflowError;
use crate::impl_workflow_step;
use crate::step::StepContext;
use chem_domain::ports::ProviderMolecule;
use chem_domain::{Molecule, MoleculeFamily};
use chem_providers::{ChemEngine, ChemEngineInterface};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
/// Input para Step5: generación de permutaciones con sustituyentes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step5Input {
  pub substitute_family_id: Option<Uuid>,
  /// Mapa opcional de puntos de unión para moléculas principales: inchikey ->
  /// atom_ids
  pub principal_join_points: Option<HashMap<String, Vec<usize>>>,
  /// Mapa opcional de puntos de unión para sustituyentes: inchikey -> atom_ids
  pub substitute_family_join_points: Option<HashMap<String, Vec<usize>>>,
  pub r_substitutes: usize,
  pub num_bounds: usize,
  pub repeat: bool,
  #[serde(default = "default_true")]
  pub save_generated: bool,
  #[serde(default)]
  pub include_principal: bool,
  #[serde(default)]
  pub permutation_limit: usize,
}
fn default_true() -> bool {
  true
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step5Params {
  pub input: Step5Input,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step5Payload {
  pub generated_for: Vec<String>,
  pub generated_molecules: Vec<String>,
  pub generated_count: usize,
  pub step_result: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step5Metadata {
  pub status: String,
  pub parameters: Step5Params,
  pub domain_refs: Vec<String>,
  pub warnings: Vec<String>,
}
#[derive(Debug, Default, Clone)]
pub struct SubstituteGenerationStep5;
impl SubstituteGenerationStep5 {
  fn load_substitute_family(&self, ctx: &StepContext, fid: Uuid) -> Result<MoleculeFamily, WorkflowError> {
    let fam =
      ctx.domain_repo
         .get_family(&fid)?
         .ok_or_else(|| WorkflowError::Validation(format!("Familia sustituyentes {} no encontrada", fid)))?;
    Ok(fam)
  }
  fn validate_input(&self, input: &Step5Input) -> Result<(), WorkflowError> {
    if input.r_substitutes == 0 {
      return Err(WorkflowError::Validation("r_substitutes debe ser > 0".into()));
    }
    if input.num_bounds == 0 || input.num_bounds > 3 {
      return Err(WorkflowError::Validation("num_bounds debe estar en 1..=3".into()));
    }
    if input.substitute_family_id.is_none() {
      return Err(WorkflowError::Validation("substitute_family_id requerido (creación dinámica aún no soportada)".into()));
    }
    Ok(())
  }
  pub fn execute_step(&self, ctx: &StepContext, input: Step5Input) -> Result<crate::step::StepInfo, WorkflowError> {
    self.validate_input(&input)?;
    // Obtener payload de Step4 (moléculas generadas)
    let step4: Option<crate::flows::cadma_flow::steps::admetsa_initial_step4::Step4Payload> =
      ctx.get_step_payload_by_name_typed("ADMETSAInitialStep4")?;
    let step4 = step4.ok_or_else(|| WorkflowError::Validation("Falta resultado de Step4".into()))?;
    let substitute_family = self.load_substitute_family(ctx, input.substitute_family_id.unwrap())?;
    let mut warnings: Vec<String> = Vec::new();
    let mut generated_for: Vec<String> = Vec::new();
    let mut generated_molecules: Vec<String> = Vec::new();
    let mut seen_inchikeys: HashSet<String> = HashSet::new();
    let mut explored: usize = 0;
    // Inicializar motor químico (RDKit)
    let engine = ChemEngine::init().map_err(|e| WorkflowError::Other(format!("Error inicializando engine: {}", e)))?;
    // Precomputar substituyentes: (mol, join_points)
    let sub_join_override = input.substitute_family_join_points.clone().unwrap_or_default();
    let mut substituents: Vec<(Molecule, Vec<usize>)> = Vec::new();
    for sm in substitute_family.molecules() {
      let smiles = sm.smiles().to_string();
      // Validar/canonizar con RDKit
      let rd = ChemEngineInterface::get_molecule(&engine, &smiles).map_err(|e| {
                                                                    WorkflowError::Other(format!("RDKit error \
                                                                                                  substituyente: {}",
                                                                                                 e))
                                                                  })?;
      let key = rd.inchikey.clone();
      let rd_points = rd.structure.as_ref().map(|s| s.substitution_points.clone()).unwrap_or_default();
      let points = if let Some(ov) = sub_join_override.get(&key) { ov.clone() } else { rd_points };
      if points.is_empty() {
        warnings.push(format!("Sustituyente {} sin puntos de unión válidos", key));
        continue;
      }
      substituents.push((sm.clone(), points));
    }
    if substituents.is_empty() {
      return Err(WorkflowError::Validation("No hay sustituyentes válidos tras validación RDKit".into()));
    }
    // Iterar moléculas objetivo
    let principal_override = input.principal_join_points.clone().unwrap_or_default();
    for ik in step4.generated_for.iter() {
      let Some(mol) = ctx.domain_repo.get_molecule(ik)? else {
        warnings.push(format!("Mol objetivo {} no encontrada en dominio", ik));
        continue;
      };
      generated_for.push(ik.clone());
      let smiles_principal = mol.smiles().to_string();
      let rd_p = ChemEngineInterface::get_molecule(&engine, &smiles_principal).map_err(|e| {
                                                                                WorkflowError::Other(format!("RDKit error \
                                                                                                              principal {}: \
                                                                                                              {}",
                                                                                                             ik, e))
                                                                              })?;
      let default_points = rd_p.structure.as_ref().map(|s| s.substitution_points.clone()).unwrap_or_default();
      let principal_points = principal_override.get(ik).cloned().unwrap_or(default_points);
      if principal_points.is_empty() {
        warnings.push(format!("Mol objetivo {} no tiene puntos de unión tras validación", ik));
        continue;
      }
      // Generación exhaustiva de PERMUTACIONES (no solo combinaciones) tanto
      // para puntos principales como para lista de sustituyentes seleccionados.
      let r_max = input.r_substitutes;
      if !input.repeat && r_max > principal_points.len() {
        warnings.push(format!("r_substitutes(max)={} > puntos_disponibles={} en {} (se limitará a {})",
                              r_max,
                              principal_points.len(),
                              ik,
                              principal_points.len()));
      }
      let effective_r_max = if !input.repeat && r_max > principal_points.len() { principal_points.len() } else { r_max };
      if input.include_principal && input.save_generated && !seen_inchikeys.contains(ik) {
        seen_inchikeys.insert(ik.clone());
        generated_molecules.push(ik.clone());
      }
      for k in 1..=effective_r_max {
        // generar todas longitudes 1..=r_max
        let principal_seqs = principal_sequences(&principal_points, k, input.repeat);
        if principal_seqs.is_empty() {
          warnings.push(format!("Sin secuencias de puntos principales para k={} en {}", k, ik));
          continue;
        }
        if principal_seqs.len() > 100_000 {
          warnings.push(format!("Advertencia: {} permutaciones de puntos principales (k={}, repeat={}) pueden impactar \
                                 rendimiento",
                                principal_seqs.len(),
                                k,
                                input.repeat));
        }
        let substituent_seqs = substituent_sequences(&substituents, k, input.repeat);
        if substituent_seqs.is_empty() {
          warnings.push(format!("Sin secuencias de sustituyentes válidas para k={} en {}", k, ik));
          continue;
        }
        if substituent_seqs.len() > 100_000 {
          warnings.push(format!("Advertencia: {} permutaciones de sustituyentes (k={}, repeat={}) pueden impactar \
                                 rendimiento",
                                substituent_seqs.len(),
                                k,
                                input.repeat));
        }
        for principal_sel in &principal_seqs {
          for subs_sel in &substituent_seqs {
            // Para cada substituyente en la tupla, obtener sus join points
            // Construir producto cartesiano de join points
            let join_points_sets: Vec<&Vec<usize>> = subs_sel.iter().map(|(_, pts)| pts).collect();
            for bond_order in 1..=input.num_bounds {
              for jp_combo in cartesian_product(&join_points_sets) {
                if input.permutation_limit > 0 && explored >= input.permutation_limit {
                  warnings.push(format!("Permutation limit {} alcanzado, deteniendo exploración",
                                        input.permutation_limit));
                  break;
                }
                // Construcción incremental: iniciar con smiles_principal
                let mut current_smiles = smiles_principal.clone();
                let mut valid_chain = true;
                let mut used_inchikey: Option<String> = None;
                // Para mapear índice principal seleccionado -> atom index actual, asumimos
                // RDKit reindex estable mientras no se cambie principal antes de fusionar.
                for (i, &p_atom) in principal_sel.iter().enumerate() {
                  let (sub_mol, sub_pts) = &subs_sel[i];
                  let jp_atom = jp_combo[i];
                  if !sub_pts.contains(&jp_atom) {
                    valid_chain = false;
                    break;
                  }
                  // Factibilidad previa: cargar RDKit dinámico de la cadena actual y sustituyente
                  let sub_rd = ChemEngineInterface::get_molecule(&engine, sub_mol.smiles().as_str())
                                     .map_err(|e| WorkflowError::Other(format!("RDKit error sub: {}", e)))?;
                  let principal_rd =
                    ChemEngineInterface::get_molecule(&engine, &current_smiles).map_err(|e| {
                                                           WorkflowError::Other(format!("RDKit error principal dinámico: \
                                                                                         {}",
                                                                                        e))
                                                         })?;
                  if !ChemEngineInterface::feasible_bond(&engine, &principal_rd, p_atom, &sub_rd, jp_atom, bond_order as u8)
                  {
                    valid_chain = false;
                    break;
                  }
                  let fused = ChemEngineInterface::fuse(&engine,
                                                        &current_smiles,
                                                        sub_rd.smiles.as_str(),
                                                        p_atom,
                                                        jp_atom,
                                                        bond_order as u8).map_err(|e| {
                                                                           WorkflowError::Other(format!("Fusión falló: {}",
                                                                                                        e))
                                                                         })?;
                  current_smiles = fused.smiles.clone();
                  used_inchikey = Some(fused.inchikey.clone());
                }
                if !valid_chain {
                  continue;
                }
                if let Some(final_ik) = used_inchikey {
                  if seen_inchikeys.contains(&final_ik) {
                    continue;
                  }
                  if input.save_generated {
                    // TODO Phase 4: Inject PropertyProvider via context
                    let engine = ChemEngine::init().map_err(|e| {
                                                     WorkflowError::Domain(chem_domain::DomainError::provider("ChemEngine",
                                                                                            format!("Init failed: {}", e)))
                                                   })?;
                    match engine.get_molecule(&current_smiles) {
                      Ok(provider_mol) => {
                        // Convert chem_providers::Molecule to chem_domain::ports::ProviderMolecule
                        let domain_provider_mol = ProviderMolecule { inchikey: provider_mol.inchikey,
                                                                     inchi: provider_mol.inchi,
                                                                     smiles: provider_mol.smiles,
                                                                     num_atoms: provider_mol.num_atoms,
                                                                     mol_weight: provider_mol.mol_weight,
                                                                     mol_formula: provider_mol.mol_formula,
                                                                     structure: None /* TODO: convert structure if
                                                                                      * needed */ };
                        match Molecule::from_provider_molecule(domain_provider_mol) {
                          Ok(new_m) => {
                            let ik_saved = ctx.domain_repo.save_molecule(new_m)?;
                            seen_inchikeys.insert(final_ik.clone());
                            generated_molecules.push(ik_saved);
                          }
                          Err(e) => warnings.push(format!("No se pudo construir molécula final: {}", e)),
                        }
                      }
                      Err(e) => warnings.push(format!("Engine error: {}", e)),
                    }
                  }
                }
                explored += 1;
              }
              if input.permutation_limit > 0 && explored >= input.permutation_limit {
                break;
              }
            }
            if input.permutation_limit > 0 && explored >= input.permutation_limit {
              break;
            }
          }
          if input.permutation_limit > 0 && explored >= input.permutation_limit {
            break;
          }
        }
        if input.permutation_limit > 0 && explored >= input.permutation_limit {
          break;
        }
      } // fin loop k
    }
    let payload = Step5Payload { generated_for: generated_for.clone(),
                                 generated_molecules: generated_molecules.clone(),
                                 generated_count: generated_molecules.len(),
                                 step_result: format!("ok (explored={})", explored) };
    let metadata =
      Step5Metadata { status: "completed".into(), parameters: Step5Params { input }, domain_refs: generated_for, warnings };
    Ok(crate::step::StepInfo { payload: serde_json::to_value(payload)?, metadata: serde_json::to_value(metadata)? })
  }
}
/// Genera TODAS las secuencias (permuta / variaciones) de longitud r a partir
/// de los puntos principales. Si repeat=false no permite reutilizar un mismo
/// punto; si repeat=true permite repeticiones (variaciones con repetición).
fn principal_sequences(points: &Vec<usize>, r: usize, repeat: bool) -> Vec<Vec<usize>> {
  let mut out = Vec::new();
  if r == 0 {
    return vec![Vec::new()];
  }
  if !repeat && r > points.len() {
    return out;
  }
  let mut current = Vec::new();
  fn backtrack(points: &Vec<usize>, r: usize, repeat: bool, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if current.len() == r {
      out.push(current.clone());
      return;
    }
    for &p in points.iter() {
      if !repeat && current.contains(&p) {
        continue;
      }
      current.push(p);
      backtrack(points, r, repeat, current, out);
      current.pop();
    }
  }
  backtrack(points, r, repeat, &mut current, &mut out);
  out
}
/// Genera TODAS las secuencias de sustituyentes de longitud r. Si repeat=false
/// equivale a permutaciones sin repetición; si repeat=true, variaciones con
/// repetición.
fn substituent_sequences(subs: &Vec<(Molecule, Vec<usize>)>, r: usize, repeat: bool) -> Vec<Vec<(Molecule, Vec<usize>)>> {
  let mut out = Vec::new();
  if r == 0 {
    return vec![Vec::new()];
  }
  if !repeat && r > subs.len() {
    return out;
  }
  let mut current: Vec<(Molecule, Vec<usize>)> = Vec::new();
  fn backtrack(subs: &Vec<(Molecule, Vec<usize>)>,
               r: usize,
               repeat: bool,
               used_idx: &mut Vec<usize>,
               current: &mut Vec<(Molecule, Vec<usize>)>,
               out: &mut Vec<Vec<(Molecule, Vec<usize>)>>) {
    if current.len() == r {
      out.push(current.clone());
      return;
    }
    for (i, item) in subs.iter().enumerate() {
      if !repeat && used_idx.contains(&i) {
        continue;
      }
      if !repeat {
        used_idx.push(i);
      }
      current.push(item.clone());
      backtrack(subs, r, repeat, used_idx, current, out);
      current.pop();
      if !repeat {
        used_idx.pop();
      }
    }
  }
  backtrack(subs, r, repeat, &mut Vec::new(), &mut current, &mut out);
  out
}
/// Producto cartesiano de un slice de vectores de índices.
fn cartesian_product(sets: &Vec<&Vec<usize>>) -> Vec<Vec<usize>> {
  let mut result: Vec<Vec<usize>> = vec![Vec::new()];
  for set in sets {
    let mut next = Vec::new();
    for prefix in &result {
      for &item in *set {
        let mut new_prefix = prefix.clone();
        new_prefix.push(item);
        next.push(new_prefix);
      }
    }
    result = next;
  }
  result
}
impl_workflow_step!(SubstituteGenerationStep5,
                    Step5Payload,
                    Step5Metadata,
                    Step5Input,
                    |this_self, ctx, input| { this_self.execute_step(ctx, input) });
