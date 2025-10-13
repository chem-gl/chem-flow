// cadma_demo.rs
//! Demo interactivo mejorado para CadmaFlow.
//! - Crea / carga flows
//! - Ejecuta pasos interactivos (Step1, Step2)
//! - Persiste resultados, guarda snapshots y maneja ramas
//! - Listar / inspeccionar datos persistidos
use chem_domain::ports::ProviderMolecule;
use chem_domain::{FamilyRepository, Molecule, MoleculeFamily, MoleculeReader, MoleculeWriter, PropertyRepository};
use chem_persistence::{new_domain_from_env, new_flow_from_env};
use chem_providers::{ChemEngine, ChemEngineInterface};
use chem_workflow::flows::cadma_flow::steps::admetsa_generated_step6::Step6Input;
use chem_workflow::flows::cadma_flow::steps::admetsa_initial_step4::{Step4Input, Step4Payload};
use chem_workflow::flows::cadma_flow::steps::admetsa_properties_step2::Step2Input;
use chem_workflow::flows::cadma_flow::steps::common::{
  ADMETSAMethod, ManualValues, PropertyValues, ALL_METHODS, REQUIRED_PROPERTIES,
};
use chem_workflow::flows::cadma_flow::steps::family_reference_step1::{Step1Input, Step1Payload};
use chem_workflow::flows::cadma_flow::steps::molecule_initial_step3::{GenerationMethod, Step3Input};
use chem_workflow::flows::cadma_flow::steps::substitute_generation_step5::Step5Input;
use chem_workflow::{factory::ChemicalWorkflowFactory, flows::cadma_flow::CadmaFlow, ChemicalFlowEngine};
use flow::repository::FlowRepository;
use serde_json::json;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use uuid::Uuid;
// Helper para crear moléculas desde SMILES usando el provider
fn molecule_from_smiles(smiles: &str) -> Result<Molecule, Box<dyn Error>> {
  let engine = ChemEngine::init()?;
  let provider_mol = engine.get_molecule(smiles)?;
  // Convertir chem_providers::Molecule a ProviderMolecule
  let converted = ProviderMolecule { inchikey: provider_mol.inchikey,
                                     inchi: provider_mol.inchi,
                                     smiles: provider_mol.smiles.clone(),
                                     num_atoms: provider_mol.num_atoms,
                                     mol_weight: provider_mol.mol_weight,
                                     mol_formula: provider_mol.mol_formula,
                                     structure: None /* Por ahora sin estructura detallada */ };
  Ok(Molecule::from_provider_molecule(converted)?)
}
fn prompt(msg: &str) -> Result<String, Box<dyn Error>> {
  print!("{}", msg);
  io::stdout().flush()?;
  let mut s = String::new();
  io::stdin().read_line(&mut s)?;
  Ok(s.trim_end().to_string())
}
fn parse_manual_values(input: &str) -> Result<PropertyValues, String> {
  let mut map = PropertyValues::new();
  for part in input.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
    let kv: Vec<&str> = part.split('=').map(|s| s.trim()).collect();
    if kv.len() != 2 {
      return Err(format!("Formato inválido en '{}'", part));
    }
    let key = kv[0].to_string();
    let val: f64 = kv[1].parse().map_err(|_| format!("Valor no numérico en '{}'", part))?;
    map.insert(key, val);
  }
  Ok(map)
}
fn get_flow_name(repo: &dyn FlowRepository, id: &Uuid) -> String {
  repo.get_flow_meta(id).ok().and_then(|m| m.name).unwrap_or_else(|| "sin nombre".to_string())
}
// Selección simple de flow desde el repo
fn select_flow_from_repo(repo: &dyn FlowRepository) -> Result<Option<Uuid>, Box<dyn Error>> {
  let ids = repo.list_flow_ids()?;
  if ids.is_empty() {
    println!("No hay flujos disponibles.");
    return Ok(None);
  }
  for (i, id) in ids.iter().enumerate() {
    println!("  [{}] {} - {}", i, id, get_flow_name(repo, id));
  }
  let s = prompt("Selecciona índice (enter para cancelar): ")?;
  if s.trim().is_empty() {
    return Ok(None);
  }
  let idx: usize = s.trim().parse()?;
  if idx >= ids.len() {
    println!("Índice fuera de rango.");
    return Ok(None);
  }
  Ok(Some(ids[idx]))
}
/// Crea un flow nuevo (persistido por factory) y devuelve la instancia cargada.
fn create_flow_interactive() -> Result<CadmaFlow, Box<dyn Error>> {
  let name = prompt("Nombre del flow (enter = cadma-demo): ")?;
  let flow_name = if name.trim().is_empty() { "cadma-demo".to_string() } else { name };
  // ChemicalWorkflowFactory::create<T> crea y persiste el flow en la repo
  let engine_box = ChemicalWorkflowFactory::create::<CadmaFlow>(flow_name)?;
  println!("Flow creado: {}", engine_box.id());
  // Unbox para devolver la instancia concreta
  Ok(*engine_box)
}
/// Muestra metadatos y flow_meta básicos
fn show_metadata(engine: &CadmaFlow) {
  match engine.get_metadata("flow_metadata") {
    Ok(meta) => println!("flow_metadata: {}", serde_json::to_string_pretty(&meta).unwrap_or_default()),
    Err(e) => println!("No hay metadata (error: {})", e),
  }
  println!("ID: {}, current_step: {}, status: {:?}",
           engine.id(),
           engine.current_step(),
           engine.status());
}
/// Ejecuta interactivamente Step1 (FamilyReferenceStep1)
fn run_step1(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  println!("\n== Step1: Familias ==");
  // Mostrar familias existentes en domain repo
  let domain = engine.domain_repo();
  let families = domain.list_families().unwrap_or_default();
  // Caso 1: no hay familias -> crear nueva con SMILES
  if families.is_empty() {
    println!("No existen familias en el dominio. Crear nueva familia con SMILES.");
    let smiles = prompt("SMILES (separados por coma): ")?;
    let mut mols = Vec::new();
    for s in smiles.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
      match molecule_from_smiles(s) {
        Ok(m) => mols.push(m),
        Err(e) => println!("SMILES inválido '{}': {}", s, e),
      }
    }
    if mols.is_empty() {
      println!("No se crearon moléculas. Abortando Step1.");
      return Ok(());
    }
    let name = prompt("Nombre de la nueva familia (opcional): ")?;
    let input = Step1Input { families: None,
                             molecules: Some(mols),
                             new_family_name: if name.trim().is_empty() { None } else { Some(name) },
                             new_family_description: None };
    // Forzamos temporalmente el current_step a 0 para permitir la ejecución
    // manual del Step1 en el demo aun cuando el flow cargado tenga
    // `current_step` avanzado. Esto evita que la validación de pasos
    // previos impida la ejecución interactiva del primer paso.
    let json_input = serde_json::to_value(&input)?;
    let info = engine.execute_step_by_index_unchecked(0, &json_input)?;
    let step_name = engine.current_step_name()?;
    engine.persist_step_result(&step_name, info, -1, None)?;
    println!("Step1 ejecutado y persistido.");
    return Ok(());
  }
  // Si hay familias, preguntar si crear nueva o seleccionar existente
  println!("Familias encontradas:");
  for (i, f) in families.iter().enumerate() {
    let name = f.name().map(|s| s.to_string()).unwrap_or_else(|| "sin nombre".to_string());
    println!("  {}: {} ({} moléculas) - id={}", i + 1, name, f.molecules().len(), f.id());
  }
  println!("0) Crear nueva familia");
  let choice = prompt("Elige número (0=create): ")?;
  if choice.trim() == "0" {
    let name = prompt("Nombre de la nueva familia: ")?;
    // Pedimos SMILES para poblarla opcionalmente
    let smiles = prompt("SMILES (opcional, coma separados): ")?;
    let mut mols_opt = None;
    if !smiles.trim().is_empty() {
      let mut mols = Vec::new();
      for s in smiles.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Ok(m) = molecule_from_smiles(s) {
          mols.push(m);
        } else {
          println!("SMILES inválido (ignorado): {}", s);
        }
      }
      if !mols.is_empty() {
        mols_opt = Some(mols);
      }
    }
    let input = Step1Input { families: None,
                             molecules: mols_opt,
                             new_family_name: if name.trim().is_empty() { None } else { Some(name) },
                             new_family_description: None };
    // Forzamos current_step a 0 por la misma razón explicada arriba.
    let json_input = serde_json::to_value(&input)?;
    let info = engine.execute_step_by_index_unchecked(0, &json_input)?;
    let step_name = "FamilyReferenceStep1".to_string();
    engine.persist_step_result(&step_name, info, -1, None)?;
    println!("Nueva familia creada y Step1 persistido.");
    return Ok(());
  }
  // seleccionar existente
  if let Ok(n) = choice.trim().parse::<usize>() {
    if n >= 1 && n <= families.len() {
      let fid = families[n - 1].id();
      let input =
        Step1Input { families: Some(vec![fid]), molecules: None, new_family_name: None, new_family_description: None };
      // Evitar doble ejecución si ya existe payload
      let step_name = engine.current_step_name()?;
      if let Ok(Some(_)) = engine.get_last_step_payload(&step_name) {
        println!("El paso ya fue ejecutado para este flow; omitiendo.");
        return Ok(());
      }
      // Forzamos current_step=0 para permitir la ejecución interactiva del
      // Step1 y evitar el error por pasos previos faltantes.
      let json_input = serde_json::to_value(&input)?;
      let info = engine.execute_step_by_index_unchecked(0, &json_input)?;
      engine.persist_step_result(&step_name, info, -1, None)?;
      println!("Familia seleccionada y Step1 persistido.");
      return Ok(());
    } else {
      println!("Índice inválido.");
    }
  }
  Ok(())
}
/// Ejecuta interactivamente Step2 (ADMETSA)
fn run_step2(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  println!("\n== Step2: ADMETSA ==");
  // Mostrar capacidades por método para ayudar la selección (recreamos
  // localmente)
  println!("Métodos disponibles (y propiedades que generan):");
  for &m in &ALL_METHODS {
    let props: Vec<String> =
      REQUIRED_PROPERTIES.iter().filter_map(|&p| if m.can_generate(p) { Some(format!("{:?}", p)) } else { None }).collect();
    println!(" - {:?} -> {}", m, props.join(", "));
  }
  // Pedimos orden preferente por nombre (coma separado) — la interfaz es
  // tolerante
  let raw = prompt("Métodos preferidos (coma separados, e.g. Random1,Random2) [enter = Random1,Random2]: ")?;
  let preferred: Vec<ADMETSAMethod> = if raw.trim().is_empty() {
    vec![ADMETSAMethod::Random1, ADMETSAMethod::Random2]
  } else {
    raw.split(',')
       .map(|s| s.trim())
       .filter_map(|tok| match tok {
         "Manual" => Some(ADMETSAMethod::Manual),
         "Random1" => Some(ADMETSAMethod::Random1),
         "Random2" => Some(ADMETSAMethod::Random2),
         "Random3" => Some(ADMETSAMethod::Random3),
         "Random4" => Some(ADMETSAMethod::Random4),
         other => {
           println!("Método desconocido: {} (ignorando)", other);
           None
         }
       })
       .collect()
  };
  // Validación local rápida: preferred cover?
  for &prop in &REQUIRED_PROPERTIES {
    let ok = preferred.iter().any(|&m| m.can_generate(prop));
    if !ok {
      println!("Los métodos preferidos no cubren la propiedad requerida: {:?}", prop);
      println!("Ajusta los métodos y vuelve a intentar.");
      return Ok(());
    }
  }
  // evitar ejecutar si Step1 no existe
  // Comprobar que Step1 ya fue ejecutado: leemos el último payload para el
  // primer paso y lo deserializamos.
  let step_name_0 = engine.step_name_by_index(0)?;
  let step1_payload = match engine.get_last_step_payload(&step_name_0)? {
    Some(v) => Some(serde_json::from_value::<Step1Payload>(v)?),
    None => None,
  };
  if step1_payload.is_none() {
    println!("No se encontró resultado de Step1: ejecuta Step1 primero.");
    return Ok(());
  }
  // Si Manual está incluido, pedir valores manuales
  let mut manual_values: Option<ManualValues> = None;
  if preferred.contains(&ADMETSAMethod::Manual) {
    // Obtener familia y moléculas
    let family = engine.domain_repo
                       .get_family(&step1_payload.as_ref().unwrap().family_uuid)?
                       .ok_or_else(|| "Familia no encontrada".to_string())?;
    let molecules: Vec<&Molecule> = family.molecules().iter().collect();
    let mut mv = ManualValues::new();
    println!("Ingresando valores manuales para {} moléculas.", molecules.len());
    println!("Propiedades requeridas: {:?}",
             REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(", "));
    for mol in &molecules {
      let smiles = mol.smiles();
      loop {
        let input = prompt(&format!("Valores para {} (formato: Prop=val,Prop=val,...): ", smiles))?;
        let parsed = parse_manual_values(&input);
        if let Ok(props) = parsed {
          // Verificar que todas las requeridas estén
          let mut missing = Vec::new();
          for &prop in &REQUIRED_PROPERTIES {
            if !props.contains_key(&format!("{:?}", prop)) {
              missing.push(format!("{:?}", prop));
            }
          }
          if !missing.is_empty() {
            println!("Faltan propiedades: {}", missing.join(", "));
            continue;
          }
          // Verificar que no haya extras (opcional, pero para ser estricto)
          let valid_keys: std::collections::HashSet<String> =
            REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect();
          let extra: Vec<String> = props.keys().filter(|k| !valid_keys.contains(*k)).cloned().collect();
          if !extra.is_empty() {
            println!("Propiedades extra no válidas: {}", extra.join(", "));
            continue;
          }
          mv.insert(smiles.to_string(), props);
          break;
        } else {
          println!("Formato inválido. Usa Prop=val,Prop=val,...");
        }
      }
    }
    manual_values = Some(mv);
  }
  // Construir input JSON y ejecutar el paso 1 (ADMETSA) sin depender del
  // `current_step` (modo interactivo). Usamos `step_name_by_index(1)` para
  // obtener el nombre correcto del paso y `execute_step_by_index_unchecked`
  // para ejecutarlo sin validar pasos previos adicionales.
  let input = Step2Input { preferred_methods: preferred, method_property_map: None, manual_values };
  let json_input = serde_json::to_value(&input)?;
  let step_idx = 1;
  let step_name = engine.step_name_by_index(step_idx)?;
  if let Ok(Some(_)) = engine.get_last_step_payload(&step_name) {
    println!("Step2 ya fue ejecutado previamente para este flow; omitiendo.");
    return Ok(());
  }
  let info = engine.execute_step_by_index_unchecked(step_idx, &json_input)?;
  engine.persist_step_result(&step_name, info, -1, None)?;
  println!("Step2 ejecutado y persistido.");
  Ok(())
}
/// Ejecuta interactivamente Step3 (Molecule Initial)
fn run_step3(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  println!("\n== Step3: Generación de Molécula Inicial ==");
  println!("Métodos disponibles:");
  println!("1) Manual: ingresar SMILES manualmente");
  println!("2) Random: usar candidatos predefinidos (c1ccccc1, CCO)");
  let choice = prompt("Elige método (1 o 2): ")?;
  let method = match choice.trim() {
    "1" => {
      let smiles = prompt("Ingresa SMILES: ")?;
      if smiles.trim().is_empty() {
        println!("SMILES vacío; abortando.");
        return Ok(());
      }
      GenerationMethod::Manual { smiles }
    }
    "2" => {
      let candidates = vec!["c1ccccc1".to_string(), "CCO".to_string()];
      GenerationMethod::Random { candidates }
    }
    _ => {
      println!("Opción inválida.");
      return Ok(());
    }
  };
  let input = Step3Input { method };
  let json_input = serde_json::to_value(&input)?;
  let step_idx = 2;
  let step_name = engine.step_name_by_index(step_idx)?;
  if let Ok(Some(_)) = engine.get_last_step_payload(&step_name) {
    println!("Step3 ya fue ejecutado previamente para este flow; omitiendo.");
    return Ok(());
  }
  let info = engine.execute_step_by_index_unchecked(step_idx, &json_input)?;
  engine.persist_step_result(&step_name, info, -1, None)?;
  println!("Step3 ejecutado y persistido.");
  Ok(())
}
/// Ejecuta interactivamente Step4 (ADMETSA para molécula(s) inicial(es))
fn run_step4(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  println!("\n== Step4: ADMETSA para molécula inicial ==");
  // Validar que existan Step2 y Step3
  let step2_name = engine.step_name_by_index(1)?;
  let step3_name = engine.step_name_by_index(2)?;
  let _step2_payload = match engine.get_last_step_payload(&step2_name)? {
    Some(v) => v,
    None => {
      println!("No se encontró resultado de Step2. Ejecuta Step2 primero.");
      return Ok(());
    }
  };
  let step3_payload = match engine.get_last_step_payload(&step3_name)? {
    Some(v) => v,
    None => {
      println!("No se encontró resultado de Step3. Ejecuta Step3 primero.");
      return Ok(());
    }
  };
  // Determinar si Step2 usó método Manual en su configuración
  use chem_workflow::flows::cadma_flow::steps::admetsa_properties_step2::Step2Metadata;
  let step2_meta_key = format!("step_state:{}", step2_name);
  let rows = engine.flow_repo().read_data(&engine.id(), 0)?;
  let mut step2_used_manual = false;
  for fd in rows.iter().rev() {
    if fd.key == step2_meta_key {
      let meta: Step2Metadata = serde_json::from_value(fd.metadata.clone())?;
      let input = meta.parameters.input;
      if input.preferred_methods.iter().any(|m| matches!(m, ADMETSAMethod::Manual))
         || input.method_property_map
                 .as_ref()
                 .map(|m| m.values().any(|mm| matches!(mm, ADMETSAMethod::Manual)))
                 .unwrap_or(false)
         || input.manual_values.is_some()
      {
        step2_used_manual = true;
      }
      break;
    }
  }
  let mut override_methods: Option<Vec<ADMETSAMethod>> = None;
  let mut manual_values: Option<ManualValues> = None;
  if step2_used_manual {
    println!("Step2 usó método Manual: puedes overridear métodos y/o cargar valores manuales para Step4.");
    let raw = prompt("Métodos override (coma separados, enter = ninguno): ")?;
    if !raw.trim().is_empty() {
      let list: Vec<ADMETSAMethod> = raw.split(',')
                                        .map(|s| s.trim())
                                        .filter_map(|tok| match tok {
                                          "Manual" => Some(ADMETSAMethod::Manual),
                                          "Random1" => Some(ADMETSAMethod::Random1),
                                          "Random2" => Some(ADMETSAMethod::Random2),
                                          "Random3" => Some(ADMETSAMethod::Random3),
                                          "Random4" => Some(ADMETSAMethod::Random4),
                                          _ => None,
                                        })
                                        .collect();
      if !list.is_empty() {
        override_methods = Some(list);
      }
    }
    // Si incluyen Manual o si quiere cargar manuales explícitamente, pedir valores
    let wants_manual =
      override_methods.as_ref().map(|v| v.iter().any(|m| matches!(m, ADMETSAMethod::Manual))).unwrap_or(false);
    if wants_manual {
      println!("Ingresa valores manuales para las moléculas generadas en Step3.");
      // Determinar las moléculas desde step3_payload
      #[derive(serde::Deserialize)]
      struct SP3 {
        generated_molecules: Vec<String>,
      }
      let sp3: SP3 = serde_json::from_value(step3_payload.clone())?;
      // Necesitamos SMILES por InChIKey
      let repo = engine.domain_repo();
      let mut mv = ManualValues::new();
      for ik in sp3.generated_molecules {
        if let Some(m) = repo.get_molecule(&ik)? {
          println!("Valores manuales para SMILES {}:", m.smiles());
          println!("Propiedades requeridas: {}",
                   REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(", "));
          loop {
            let input = prompt("Prop=val,Prop=val,... : ")?;
            let parsed = parse_manual_values(&input);
            if let Ok(props) = parsed {
              let valid: std::collections::HashSet<String> =
                REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect();
              let missing: Vec<String> = valid.iter().filter(|k| !props.contains_key(*k)).cloned().collect();
              if !missing.is_empty() {
                println!("Faltan propiedades: {}", missing.join(", "));
                continue;
              }
              mv.insert(m.smiles().to_string(), props);
              break;
            } else {
              println!("Formato inválido.");
            }
          }
        }
      }
      manual_values = Some(mv);
    }
  } else {
    println!("Step2 no usó Manual: Step4 reutilizará los métodos de Step2 (sin override).");
  }
  let input = Step4Input { override_methods, manual_values };
  let json_input = serde_json::to_value(&input)?;
  let step_idx = 3;
  let step_name = engine.step_name_by_index(step_idx)?;
  if let Ok(Some(_)) = engine.get_last_step_payload(&step_name) {
    println!("Step4 ya fue ejecutado previamente; omitiendo.");
    return Ok(());
  }
  let info = engine.execute_step_by_index_unchecked(step_idx, &json_input)?;
  engine.persist_step_result(&step_name, info, -1, None)?;
  println!("Step4 ejecutado y persistido.");
  Ok(())
}
/// Crea una rama desde un cursor especificado por el usuario
fn create_branch_from_engine(engine: &CadmaFlow) -> Result<(), Box<dyn Error>> {
  let flow_repo = engine.flow_repo();
  let flow_id = engine.id();
  let meta = flow_repo.get_flow_meta(&flow_id)?;
  let current_cursor = meta.current_cursor;
  println!("Cursor actual: {}", current_cursor);
  let cursor_str = prompt("Ingresa el cursor desde donde crear la rama (debe ser <= {}): ")?;
  let branch_cursor: i64 = cursor_str.trim().parse().map_err(|_| "Cursor inválido, debe ser un número entero")?;
  if branch_cursor > current_cursor {
    return Err("El cursor especificado es mayor que el cursor actual, no se puede ramificar desde un cursor no ejecutado".into());
  }
  if branch_cursor < 0 {
    return Err("El cursor debe ser >= 0".into());
  }
  let branch_name = format!("branch_from_{}", branch_cursor);
  let metadata = json!({"name": branch_name});
  let branch_id = flow_repo.create_branch(&flow_id, branch_cursor, metadata)?;
  println!("Rama creada: {} desde cursor {}", branch_id, branch_cursor);
  Ok(())
}
/// Carga un flow existente seleccionando desde el repo
fn load_flow_interactive() -> Result<Option<CadmaFlow>, Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  let repo_arc = Arc::new(repo);
  if let Some(flow_id) = select_flow_from_repo(&*repo_arc)? {
    match ChemicalWorkflowFactory::load::<CadmaFlow>(&flow_id) {
      Ok(loaded_box) => {
        println!("Flow cargado: {} (current_step={})", flow_id, loaded_box.current_step());
        return Ok(Some(*loaded_box));
      }
      Err(e) => {
        println!("Error cargando flow con factory: {}, intentando carga manual sin snapshot", e);
        // Carga manual: crear engine y aplicar snapshot si existe
        let domain_repo = new_domain_from_env()?;
        let mut engine = CadmaFlow::construct_with_repos(flow_id, repo_arc.clone(), Arc::new(domain_repo));
        // Intentar cargar y aplicar el último snapshot
        match repo_arc.load_latest_snapshot(&flow_id) {
          Ok(Some(snapshot_meta)) => match repo_arc.load_snapshot(&snapshot_meta.id) {
            Ok((data, _)) => match serde_json::from_slice(&data) {
              Ok(snapshot_json) => {
                if let Err(e2) = engine.apply_snapshot(&snapshot_json) {
                  println!("Error aplicando snapshot: {}, continuando sin él", e2);
                } else {
                  println!("Snapshot aplicado exitosamente");
                }
              }
              Err(e2) => println!("Error parseando snapshot JSON: {}, continuando sin él", e2),
            },
            Err(e2) => println!("Error cargando datos del snapshot: {}, continuando sin él", e2),
          },
          Ok(None) => println!("No hay snapshot disponible, cargando desde registros"),
          Err(e2) => println!("Error obteniendo snapshot: {}, continuando sin él", e2),
        }
        // Rehidratar desde registros de flow_data si es necesario (el engine puede
        // hacerlo internamente)
        println!("Flow cargado manualmente: {} (current_step={})", flow_id, engine.current_step());
        return Ok(Some(engine));
      }
    }
  }
  Ok(None)
}
/// Mostrar registros persistidos (flow_data) para el flow actual
fn dump_flow_data(engine: &CadmaFlow) -> Result<(), Box<dyn Error>> {
  let repo = engine.flow_repo();
  let rows = repo.read_data(&engine.id(), 0)?;
  println!("Registros persistidos ({}):", rows.len());
  for r in rows {
    println!(" cursor={} key={} payload={}", r.cursor, r.key, r.payload);
  }
  Ok(())
}
fn list_families() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  let fams = repo.list_families()?;
  println!("Familias encontradas: {}", fams.len());
  for f in fams {
    println!(" - {} ({} moléculas) id={}",
             f.name().map(|s| s.to_string()).unwrap_or_default(),
             f.molecules().len(),
             f.id());
  }
  Ok(())
}

/// Listar familias con detalle de moléculas
fn list_families_detailed() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  let fams = match repo.list_families() {
    Ok(f) => f,
    Err(e) => {
      println!("Error al listar familias: {}", e);
      println!("Posible problema de migración de base de datos.");
      return Ok(());
    }
  };

  if fams.is_empty() {
    println!("No hay familias en el dominio.");
    return Ok(());
  }

  println!("\n=== Familias encontradas: {} ===", fams.len());
  for (i, f) in fams.iter().enumerate() {
    let name = f.name().map(|s| s.to_string()).unwrap_or_else(|| "sin nombre".to_string());
    println!("\n[{}] Familia: {} (ID: {})", i, name, f.id());
    println!("    Moléculas ({}): ", f.molecules().len());

    for (j, mol) in f.molecules().iter().enumerate() {
      println!("      [{}] InChIKey: {} | SMILES: {}", j, mol.inchikey(), mol.smiles());
    }
  }

  let view_detail = prompt("\n¿Ver detalle de una familia? (índice o enter para cancelar): ")?;
  if !view_detail.trim().is_empty() {
    if let Ok(idx) = view_detail.trim().parse::<usize>() {
      if idx < fams.len() {
        let family = &fams[idx];
        let name = family.name().map(|s| s.to_string()).unwrap_or_else(|| "sin nombre".to_string());
        println!("\n=== Detalle de Familia: {} ===", name);
        println!("ID: {}", family.id());
        println!("\nMoléculas:");

        for (j, mol) in family.molecules().iter().enumerate() {
          println!("\n  [{}] Molécula:", j);
          println!("      InChIKey: {}", mol.inchikey());
          println!("      SMILES:   {}", mol.smiles());
          println!("      InChI:    {}", mol.inchi());
          if let Some(formula) = mol.molecular_formula() {
            println!("      Fórmula:  {}", formula);
          }
          if let Some(weight) = mol.estimated_molecular_weight() {
            println!("      Peso Mol: {:.2}", weight);
          }
        }
      } else {
        println!("Índice fuera de rango.");
      }
    }
  }

  Ok(())
}
fn list_flows() -> Result<(), Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  let ids = repo.list_flow_ids()?;
  println!("Flujos encontrados: {}", ids.len());
  for id in ids {
    println!(" - {} - {}", id, get_flow_name(&repo, &id));
  }
  Ok(())
}
/// Ver una molécula y sus propiedades almacenadas (versión interactiva
/// mejorada)
fn view_molecule_interactive() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;

  let mols = match repo.list_molecules() {
    Ok(m) => m,
    Err(e) => {
      println!("Error al listar moléculas: {}", e);
      println!("Posible problema de migración de base de datos.");
      println!("Intenta recrear la base de datos o ejecuta las migraciones pendientes.");
      return Ok(());
    }
  };

  if mols.is_empty() {
    println!("No hay moléculas en el dominio.");
    return Ok(());
  }

  println!("\n=== Moléculas disponibles ({}) ===", mols.len());
  for (i, m) in mols.iter().enumerate() {
    let formula = m.molecular_formula().map(|f| f.to_string()).unwrap_or_else(|| "N/A".to_string());
    let weight = m.estimated_molecular_weight().map(|w| format!("{:.2}", w)).unwrap_or_else(|| "N/A".to_string());
    println!("  [{}] {} | SMILES: {} | Fórmula: {} | Peso: {}",
             i,
             m.inchikey(),
             m.smiles(),
             formula,
             weight);
  }

  let input = prompt("\nSelecciona índice de molécula (o enter para cancelar): ")?;
  if input.trim().is_empty() {
    return Ok(());
  }

  let idx: usize = input.trim().parse().map_err(|_| "Índice inválido")?;
  if idx >= mols.len() {
    println!("Índice fuera de rango.");
    return Ok(());
  }

  let m = &mols[idx];
  let inchikey = m.inchikey();

  println!("\n=== Detalle de Molécula ===");
  println!("InChIKey: {}", m.inchikey());
  println!("SMILES:   {}", m.smiles());
  println!("InChI:    {}", m.inchi());
  if let Some(formula) = m.molecular_formula() {
    println!("Fórmula:  {}", formula);
  }
  if let Some(weight) = m.estimated_molecular_weight() {
    println!("Peso Mol: {:.2}", weight);
  }
  println!("Metadata: {}",
           serde_json::to_string_pretty(m.metadata()).unwrap_or_else(|_| "{}".to_string()));

  match repo.get_molecular_properties(inchikey.as_str()) {
    Ok(props) => {
      if props.is_empty() {
        println!("\nNo hay propiedades moleculares registradas.");
      } else {
        println!("\n=== Propiedades Moleculares ({}) ===", props.len());
        for (i, p) in props.iter().enumerate() {
          let val = serde_json::to_string_pretty(&p.value).unwrap_or_else(|_| "<no-json>".into());
          println!("  [{}] Tipo: {}", i, p.property_type);
          println!("      Calidad: {}", p.quality.as_deref().unwrap_or("-"));
          println!("      Valor: {}", val);
          println!("      Metadata: {}",
                   serde_json::to_string_pretty(&p.metadata).unwrap_or_default());
        }
      }
    }
    Err(e) => {
      println!("\n⚠️  Error obteniendo propiedades: {}", e);
      println!("Las propiedades no se pudieron cargar, pero la molécula existe.");
    }
  }

  Ok(())
}

/// Crear familia interactivamente seleccionando moléculas del dominio
fn create_family_interactive() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;

  println!("\n=== Crear nueva familia ===");
  let family_name = prompt("Nombre de la familia (opcional): ")?;
  let family_desc = prompt("Descripción de la familia (opcional): ")?;

  let mols = match repo.list_molecules() {
    Ok(m) => m,
    Err(e) => {
      println!("Error al listar moléculas: {}", e);
      println!("Posible problema de migración de base de datos.");
      return Ok(());
    }
  };

  if mols.is_empty() {
    println!("No hay moléculas disponibles en el dominio.");
    println!("Crea moléculas primero (opción 13 del menú).");
    return Ok(());
  }

  println!("\n=== Moléculas disponibles ({}) ===", mols.len());
  for (i, m) in mols.iter().enumerate() {
    let formula = m.molecular_formula().map(|f| f.to_string()).unwrap_or_else(|| "N/A".to_string());
    println!("  [{}] {} | SMILES: {} | Fórmula: {}", i, m.inchikey(), m.smiles(), formula);
  }

  println!("\nSelecciona las moléculas para la familia:");
  println!("  - Ingresa índices separados por comas (ej: 0,2,5)");
  println!("  - O ingresa 'all' para seleccionar todas");
  let selection = prompt("Selección: ")?;

  let selected_mols: Vec<Molecule> = if selection.trim().eq_ignore_ascii_case("all") {
    mols.clone()
  } else {
    let mut selected = Vec::new();
    for idx_str in selection.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
      match idx_str.parse::<usize>() {
        Ok(idx) => {
          if idx < mols.len() {
            selected.push(mols[idx].clone());
          } else {
            println!("Índice {} fuera de rango, ignorado.", idx);
          }
        }
        Err(_) => {
          println!("'{}' no es un índice válido, ignorado.", idx_str);
        }
      }
    }
    selected
  };

  if selected_mols.is_empty() {
    println!("No se seleccionaron moléculas válidas; abortando creación de familia.");
    return Ok(());
  }

  println!("\nMoléculas seleccionadas: {}", selected_mols.len());
  for m in &selected_mols {
    println!("  - {} ({})", m.smiles(), m.inchikey());
  }

  let confirm = prompt("\n¿Confirmar creación de familia? (y/N): ")?;
  if !matches!(confirm.trim().to_lowercase().as_str(), "y" | "s" | "si" | "yes") {
    println!("Creación de familia cancelada.");
    return Ok(());
  }

  let mut metadata = serde_json::json!({});
  if !family_name.trim().is_empty() {
    metadata["name"] = serde_json::json!(family_name.trim());
  }
  if !family_desc.trim().is_empty() {
    metadata["description"] = serde_json::json!(family_desc.trim());
  }

  let fam = MoleculeFamily::new(selected_mols, metadata)?;
  let family_id = repo.save_family(fam)?;

  println!("✅ Familia creada exitosamente con ID: {}", family_id);

  Ok(())
}

/// Ver una molécula y sus propiedades almacenadas
fn view_molecule() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  let input = prompt("InChIKey de la molécula (enter para listar): ")?;
  let inchikey = if input.trim().is_empty() {
    let mols = repo.list_molecules()?;
    if mols.is_empty() {
      println!("No hay moléculas en el dominio.");
      return Ok(());
    }
    println!("Moléculas disponibles ({}):", mols.len());
    for (i, m) in mols.iter().enumerate() {
      println!("  [{}] {}  | SMILES={} ", i, m.inchikey(), m.smiles());
    }
    let s = prompt("Selecciona índice: ")?;
    let idx: usize = s.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= mols.len() {
      println!("Índice fuera de rango.");
      return Ok(());
    }
    mols[idx].inchikey().to_string()
  } else {
    input.trim().to_uppercase()
  };
  match repo.get_molecule(&inchikey) {
    Ok(Some(m)) => {
      println!("\n== Molécula ==");
      println!("InChIKey: {}", m.inchikey());
      println!("SMILES:   {}", m.smiles());
      println!("InChI:    {}", m.inchi());
      println!("Metadata: {}",
               serde_json::to_string_pretty(m.metadata()).unwrap_or_else(|_| "{}".to_string()));
      match repo.get_molecular_properties(m.inchikey().as_str()) {
        Ok(props) => {
          println!("\nPropiedades ({}):", props.len());
          for (i, p) in props.iter().enumerate() {
            let val = serde_json::to_string_pretty(&p.value).unwrap_or_else(|_| "<no-json>".into());
            println!("  [{}] tipo={}  calidad={}  valor={} ",
                     i,
                     p.property_type,
                     p.quality.as_deref().unwrap_or("-"),
                     val);
          }
        }
        Err(e) => println!("Error obteniendo propiedades: {}", e),
      }
    }
    Ok(None) => println!("No se encontró la molécula con InChIKey: {}", inchikey),
    Err(e) => println!("Error consultando molécula: {}", e),
  }
  Ok(())
}
fn save_snapshot(engine: &CadmaFlow) {
  match engine.save_snapshot() {
    Ok(_) => println!("Snapshot guardado (best-effort)."),
    Err(e) => println!("Error guardando snapshot: {}", e),
  }
}
/// Ejecuta interactivamente Step5 (Generación de sustituciones)
fn run_step5(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  // Step5 interactive execution
  println!("\n== Step5: Generación de sustituciones ==");
  // Necesita Step4
  let step4_name = engine.step_name_by_index(3)?; // Step4 index
  let step4_payload_val = match engine.get_last_step_payload(&step4_name)? {
    Some(v) => v,
    None => {
      println!("No se encontró resultado de Step4. Ejecuta Step4 primero.");
      return Ok(());
    }
  };
  let step4_payload: Step4Payload = serde_json::from_value(step4_payload_val)?;
  // Seleccionar/crear familia de sustituyentes
  let domain = engine.domain_repo();
  let families = domain.list_families()?;

  if families.is_empty() {
    println!("No hay familias disponibles. Debes crear una familia de substituyentes primero.");
    println!("Usa la opción 15 del menú principal para crear una familia.");
    return Ok(());
  }

  println!("\n=== Familias disponibles (para elegir substituyentes) ===");
  for (i, f) in families.iter().enumerate() {
    let name = f.name().map(|s| s.to_string()).unwrap_or_else(|| "sin nombre".to_string());
    println!("  [{}] {} ({} moléculas) - ID: {}", i, name, f.molecules().len(), f.id());
    println!("      Moléculas:");
    for (j, mol) in f.molecules().iter().enumerate().take(3) {
      println!("        [{}] {}", j, mol.smiles());
    }
    if f.molecules().len() > 3 {
      println!("        ... y {} más", f.molecules().len() - 3);
    }
  }
  println!("  [n] Crear nueva familia de substituyentes");

  let choice = prompt("\nElige índice o 'n' para crear nueva: ")?;
  let substituent_family_id = if choice.trim().eq_ignore_ascii_case("n") {
    println!("\n=== Crear nueva familia de substituyentes ===");
    println!("Opciones:");
    println!("  1) Ingresar SMILES manualmente");
    println!("  2) Seleccionar desde moléculas existentes");

    let option = prompt("Elige opción (1 o 2): ")?;

    match option.trim() {
      "1" => {
        let smiles_line = prompt("SMILES substituyentes (separados por comas): ")?;
        let mut mols = Vec::new();
        for s in smiles_line.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
          match molecule_from_smiles(s) {
            Ok(m) => {
              println!("  ✓ Molécula creada: {}", s);
              mols.push(m);
            }
            Err(e) => println!("  ✗ SMILES inválido '{}' ({}) ignorado", s, e),
          }
        }
        if mols.is_empty() {
          println!("No se crearon moléculas para la nueva familia. Abortando Step5.");
          return Ok(());
        }
        let fam = MoleculeFamily::new(mols, serde_json::json!({"source":"step5_substitutes"}))?;
        let fid = domain.save_family(fam)?;
        println!("✅ Familia de substituyentes creada con ID: {}", fid);
        fid
      }
      "2" => {
        let all_mols = domain.list_molecules()?;
        if all_mols.is_empty() {
          println!("No hay moléculas disponibles. Crea moléculas primero.");
          return Ok(());
        }

        println!("\n=== Moléculas disponibles ({}) ===", all_mols.len());
        for (i, m) in all_mols.iter().enumerate() {
          println!("  [{}] {} | SMILES: {}", i, m.inchikey(), m.smiles());
        }

        let selection = prompt("\nÍndices separados por comas (o 'all'): ")?;
        let selected: Vec<Molecule> = if selection.trim().eq_ignore_ascii_case("all") {
          all_mols.clone()
        } else {
          let mut sel = Vec::new();
          for idx_str in selection.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if let Ok(idx) = idx_str.parse::<usize>() {
              if idx < all_mols.len() {
                sel.push(all_mols[idx].clone());
              }
            }
          }
          sel
        };

        if selected.is_empty() {
          println!("No se seleccionaron moléculas. Abortando Step5.");
          return Ok(());
        }

        println!("Moléculas seleccionadas: {}", selected.len());
        let fam = MoleculeFamily::new(selected, serde_json::json!({"source":"step5_substitutes"}))?;
        let fid = domain.save_family(fam)?;
        println!("✅ Familia de substituyentes creada con ID: {}", fid);
        fid
      }
      _ => {
        println!("Opción inválida.");
        return Ok(());
      }
    }
  } else {
    let idx: usize = match choice.trim().parse() {
      Ok(v) => v,
      Err(_) => {
        println!("Entrada inválida.");
        return Ok(());
      }
    };
    if idx >= families.len() {
      println!("Índice fuera de rango.");
      return Ok(());
    }
    families[idx].id()
  };
  // Parámetros base
  let r_sub = prompt("Máximo número de sustituyentes a insertar (r_substitutes, entero >0): ")?;
  let r_substitutes: usize = r_sub.trim().parse().unwrap_or(1);
  let nb = prompt("Máximo orden de enlace a explorar (num_bounds 1..3) [1]: ")?;
  let num_bounds: usize = if nb.trim().is_empty() { 1 } else { nb.trim().parse().unwrap_or(1) };
  let repeat_ans = prompt("Permitir reutilizar puntos/sustituyentes (repeat) [n]: ")?;
  let repeat = matches!(repeat_ans.trim().to_lowercase().as_str(), "y" | "s" | "si" | "yes");
  let save_ans = prompt("Guardar moléculas generadas en dominio? [Y/n]: ")?;
  let save_generated = !matches!(save_ans.trim().to_lowercase().as_str(), "n" | "no");
  // Overrides de puntos de unión principal
  println!("Puedes especificar puntos de sustitución (índices de átomos) para las moléculas principales.");
  println!("Deja vacío para usar los detectados automáticamente por RDKit (átomos con hidrógenos disponibles).");
  let mut principal_join_points: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
  for ik in &step4_payload.generated_for {
    if let Some(m) = domain.get_molecule(ik)? {
      let ans = prompt(&format!("Puntos para principal {} (SMILES {}): ", ik, m.smiles()))?;
      if !ans.trim().is_empty() {
        let pts: Vec<usize> =
          ans.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
        if !pts.is_empty() {
          principal_join_points.insert(ik.clone().to_string(), pts);
        }
      }
    }
  }
  if principal_join_points.is_empty() {
    println!("Usando puntos automáticos RDKit para principales.");
  }
  // Overrides de puntos de unión para substituyentes (sobre InChIKey)
  println!("Overrides de puntos para sustituyentes (por InChIKey). Deja vacío para usar automáticos.");
  let sub_family = domain.get_family(&substituent_family_id)?.ok_or("Familia no encontrada tras crearla")?;
  let mut substitute_family_join_points: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
  for sm in sub_family.molecules() {
    let ik = sm.inchikey().to_string();
    let ans = prompt(&format!("Puntos para sustituyente {} (SMILES {}): ", ik, sm.smiles()))?;
    if !ans.trim().is_empty() {
      let pts: Vec<usize> =
        ans.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
      if !pts.is_empty() {
        substitute_family_join_points.insert(ik, pts);
      }
    }
  }
  if substitute_family_join_points.is_empty() {
    println!("Usando puntos automáticos RDKit para sustituyentes.");
  }
  let input =
    Step5Input { substitute_family_id: Some(substituent_family_id),
                 principal_join_points: if principal_join_points.is_empty() { None } else { Some(principal_join_points) },
                 substitute_family_join_points: if substitute_family_join_points.is_empty() {
                   None
                 } else {
                   Some(substitute_family_join_points)
                 },
                 r_substitutes,
                 num_bounds,
                 repeat,
                 save_generated,
                 include_principal: true,
                 permutation_limit: 0 };
  let step_idx = 4; // Step5 index
  let step_name = engine.step_name_by_index(step_idx)?;
  if let Ok(Some(_)) = engine.get_last_step_payload(&step_name) {
    let rerun = prompt("Step5 ya tiene un resultado previo. Re-ejecutar? (y/N): ")?;
    if !matches!(rerun.trim().to_lowercase().as_str(), "y" | "s" | "si" | "yes") {
      println!("Omitiendo ejecución de Step5.");
      return Ok(());
    }
  }
  let json_input = serde_json::to_value(&input)?;
  let info = engine.execute_step_by_index_unchecked(step_idx, &json_input)?;
  engine.persist_step_result(&step_name, info, -1, None)?;
  println!("Step5 ejecutado y persistido.");
  Ok(())
}
/// Ejecuta interactivamente Step6 (ADMETSA sobre moléculas generadas en Step5)
fn run_step6(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  use chem_workflow::flows::cadma_flow::steps::substitute_generation_step5::Step5Payload;
  println!("\n== Step6: ADMETSA para moléculas generadas en Step5 ==");
  let step5_name = engine.step_name_by_index(4)?; // index Step5
  let step5_payload_val = match engine.get_last_step_payload(&step5_name)? {
    Some(v) => v,
    None => {
      println!("No se encontró resultado de Step5. Ejecuta Step5 primero.");
      return Ok(());
    }
  };
  let step5_payload: Step5Payload = serde_json::from_value(step5_payload_val)?;
  if step5_payload.generated_molecules.is_empty() {
    println!("Step5 no generó moléculas.");
    return Ok(());
  }
  let ov_raw = prompt("Métodos override (coma, enter = ninguno): ")?;
  let override_methods: Option<Vec<ADMETSAMethod>> = if ov_raw.trim().is_empty() {
    None
  } else {
    let v: Vec<ADMETSAMethod> = ov_raw.split(',')
                                      .map(|s| s.trim())
                                      .filter_map(|tok| match tok {
                                        "Manual" => Some(ADMETSAMethod::Manual),
                                        "Random1" => Some(ADMETSAMethod::Random1),
                                        "Random2" => Some(ADMETSAMethod::Random2),
                                        "Random3" => Some(ADMETSAMethod::Random3),
                                        "Random4" => Some(ADMETSAMethod::Random4),
                                        _ => None,
                                      })
                                      .collect();
    if v.is_empty() {
      None
    } else {
      Some(v)
    }
  };
  let mut manual_values: Option<ManualValues> = None;
  if override_methods.as_ref().map(|m| m.iter().any(|mm| matches!(mm, ADMETSAMethod::Manual))).unwrap_or(false) {
    println!("Override incluye Manual: puedes proporcionar valores manuales.");
    let mut mv = ManualValues::new();
    println!("Propiedades requeridas: {}",
             REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(", "));
    for ik in &step5_payload.generated_molecules {
      if let Some(m) = engine.domain_repo().get_molecule(ik)? {
        let ans = prompt(&format!("Valores para {} (Prop=val,...) [enter salta]: ", m.smiles()))?;
        if ans.trim().is_empty() {
          continue;
        }
        match parse_manual_values(&ans) {
          Ok(map) => {
            mv.insert(m.smiles().to_string(), map);
          }
          Err(e) => {
            println!("Formato inválido: {}", e);
          }
        }; // end match
      }
    }
    if !mv.is_empty() {
      manual_values = Some(mv);
    }
  }
  let input = Step6Input { override_methods, manual_values };
  let step_idx = 5; // Step6 index
  let step_name = engine.step_name_by_index(step_idx)?;
  let json_input = serde_json::to_value(&input)?;
  let info = engine.execute_step_by_index_unchecked(step_idx, &json_input)?;
  engine.persist_step_result(&step_name, info, -1, None)?;
  println!("Step6 ejecutado y persistido.");
  Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
  println!("🚀 CadmaFlow Interactive Demo (mejorado)");
  // Inicializar repositorio (verificamos configuración)
  let _flow_repo = match new_flow_from_env() {
    Ok(r) => Arc::new(r) as Arc<dyn FlowRepository>,
    Err(e) => {
      eprintln!("No se pudo inicializar flow repo: {}", e);
      return Err(Box::new(e));
    }
  };
  // Estado del engine en memoria (podemos crear o cargar)
  let mut maybe_engine: Option<CadmaFlow> = None;
  loop {
    println!("\n== Menú principal ==");
    println!("1) Crear flow nuevo");
    println!("2) Cargar flow existente");
    println!("3) Mostrar metadata / estado del flow cargado");
    println!("4) Ejecutar Step1 (Family)");
    println!("5) Ejecutar Step2 (ADMETSA)");
    println!("6) Ejecutar Step3 (Molecule Initial)");
    println!("7) Ejecutar Step4 (ADMETSA para molécula inicial)");
    println!("8) Ejecutar Step5 (Generación de sustituciones)");
    println!("9) Ejecutar Step6 (ADMETSA para generadas Step5)");
    println!("10) Crear rama desde cursor especificado");
    println!("11) Dump flow_data (registros persistidos)");
    println!("12) Listar familias (dominio)");
    println!("13) Crear molécula (persistir en dominio)");
    println!("14) Ver molécula (detalle y propiedades)");
    println!("15) Crear familia desde moléculas existentes en dominio");
    println!("16) Listar todos los flujos");
    println!("0) Guardar snapshot");
    println!("q) Salir");
    let opt = prompt("Opción: ")?;
    match opt.as_str() {
      "1" => match create_flow_interactive() {
        Ok(engine) => {
          maybe_engine = Some(engine);
        }
        Err(e) => println!("Error creando flow: {}", e),
      },
      "2" => {
        if let Some(engine) = load_flow_interactive()? {
          maybe_engine = Some(engine)
        }
      }
      "3" => {
        if let Some(engine) = &maybe_engine {
          show_metadata(engine);
        } else {
          println!("No hay flow cargado en memoria.");
        }
      }
      "4" => {
        if let Some(engine) = maybe_engine.as_mut() {
          // Manejar el posible error en lugar de `unwrap()` que paniquea.
          if let Err(e) = run_step1(engine) {
            println!("Error ejecutando Step1: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "5" => {
        if let Some(engine) = maybe_engine.as_mut() {
          if let Err(e) = run_step2(engine) {
            println!("Error en Step2: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "6" => {
        if let Some(engine) = maybe_engine.as_mut() {
          if let Err(e) = run_step3(engine) {
            println!("Error en Step3: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "7" => {
        if let Some(engine) = maybe_engine.as_mut() {
          if let Err(e) = run_step4(engine) {
            println!("Error en Step4: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "8" => {
        if let Some(engine) = maybe_engine.as_mut() {
          if let Err(e) = run_step5(engine) {
            println!("Error en Step5: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "9" => {
        if let Some(engine) = maybe_engine.as_mut() {
          if let Err(e) = run_step6(engine) {
            println!("Error en Step6: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "10" => {
        if let Some(engine) = &maybe_engine {
          if let Err(e) = create_branch_from_engine(engine) {
            println!("Error creando rama: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "11" => {
        if let Some(engine) = &maybe_engine {
          if let Err(e) = dump_flow_data(engine) {
            println!("Error volcando flow_data: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "12" => {
        if let Err(e) = list_families_detailed() {
          println!("Error listando familias: {}", e);
        }
      }
      "13" => {
        // Crear molécula (persistir en dominio)
        let repo = match new_domain_from_env() {
          Ok(r) => r,
          Err(e) => {
            println!("No se pudo inicializar domain repo: {}", e);
            continue;
          }
        };
        let smiles = prompt("SMILES de la nueva molécula: ")?;
        if smiles.trim().is_empty() {
          println!("SMILES vacío; abortando.");
          continue;
        }
        match molecule_from_smiles(&smiles) {
          Ok(m) => match repo.save_molecule(m.clone()) {
            Ok(key) => println!("Molécula creada y guardada con inchikey: {}", key),
            Err(e) => println!("Error guardando molécula: {}", e),
          },
          Err(e) => println!("Error creando molécula desde SMILES: {}", e),
        }
      }
      "14" => {
        if let Err(e) = view_molecule_interactive() {
          println!("Error viendo molécula: {}", e);
        }
      }
      "15" => {
        // Crear familia desde moléculas existentes en dominio (con selección
        // interactiva)
        if let Err(e) = create_family_interactive() {
          println!("Error creando familia: {}", e);
        }
      }
      "16" => {
        if let Err(e) = list_flows() {
          println!("Error listando flujos: {}", e);
        }
      }
      "0" => {
        if let Some(engine) = &maybe_engine {
          save_snapshot(engine);
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "q" | "Q" => {
        println!("👋 Saliendo.");
        break;
      }
      other => println!("Opción no válida: {}", other),
    }
  }
  Ok(())
}
