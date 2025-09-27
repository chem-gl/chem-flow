// cadma_demo.rs
//! Demo interactivo mejorado para CadmaFlow.
//! - Crea / carga flows
//! - Ejecuta pasos interactivos (Step1, Step2)
//! - Persiste resultados, guarda snapshots y maneja ramas
//! - Listar / inspeccionar datos persistidos
use chem_domain::{DomainRepository, Molecule, MoleculeFamily};
use chem_persistence::{new_domain_from_env, new_flow_from_env};
use chem_workflow::flows::cadma_flow::steps::{
  admetsa_properties_step2::{ADMETSAMethod, ManualValues, PropertyValues, Step2Input, ALL_METHODS, REQUIRED_PROPERTIES},
  family_reference_step1::{Step1Input, Step1Payload},
  molecule_initial_step3::{GenerationMethod, Step3Input},
};
use chem_workflow::{factory::ChemicalWorkflowFactory, flows::cadma_flow::CadmaFlow, ChemicalFlowEngine};
use flow::repository::FlowRepository;
use serde_json::json;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use uuid::Uuid;

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
      match Molecule::from_smiles(s) {
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
        if let Ok(m) = Molecule::from_smiles(s) {
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
    println!("Propiedades requeridas: {:?}", REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>().join(", "));
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
          let valid_keys: std::collections::HashSet<String> = REQUIRED_PROPERTIES.iter().map(|p| format!("{:?}", p)).collect();
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
}/// Carga un flow existente seleccionando desde el repo
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
          Ok(Some(snapshot_meta)) => {
            match repo_arc.load_snapshot(&snapshot_meta.id) {
              Ok((data, _)) => {
                match serde_json::from_slice(&data) {
                  Ok(snapshot_json) => {
                    if let Err(e2) = engine.apply_snapshot(&snapshot_json) {
                      println!("Error aplicando snapshot: {}, continuando sin él", e2);
                    } else {
                      println!("Snapshot aplicado exitosamente");
                    }
                  }
                  Err(e2) => println!("Error parseando snapshot JSON: {}, continuando sin él", e2),
                }
              }
              Err(e2) => println!("Error cargando datos del snapshot: {}, continuando sin él", e2),
            }
          }
          Ok(None) => println!("No hay snapshot disponible, cargando desde registros"),
          Err(e2) => println!("Error obteniendo snapshot: {}, continuando sin él", e2),
        }
        // Rehidratar desde registros de flow_data si es necesario (el engine puede hacerlo internamente)
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

/// Crear una molécula y guardarla en el repositorio de dominio
fn create_molecule_interactive() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  let smiles = prompt("SMILES de la molécula: ")?;
  if smiles.trim().is_empty() {
    println!("SMILES vacío; abortando.");
    return Ok(());
  }
  match Molecule::from_smiles(&smiles) {
    Ok(m) => match repo.save_molecule(m.clone()) {
      Ok(inchikey) => {
        println!("Molécula guardada con InChIKey: {}", inchikey);
        Ok(())
      }
      Err(e) => Err(Box::new(e)),
    },
    Err(e) => {
      println!("Error creando molécula desde SMILES: {}", e);
      Ok(())
    }
  }
}

/// Crear una familia a partir de moléculas existentes en el repo
fn create_family_from_molecules_interactive() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  let mols = repo.list_molecules()?;
  if mols.is_empty() {
    println!("No hay moléculas en el repositorio. Crea primero algunas moléculas.");
    return Ok(());
  }
  println!("Moléculas disponibles:");
  for (i, m) in mols.iter().enumerate() {
    println!("  {}: {} - {}", i + 1, m.smiles(), m.inchikey());
  }
  let raw = prompt("Indices de moléculas para la familia (coma separados, e.g. 1,3): ")?;
  if raw.trim().is_empty() {
    println!("No se seleccionaron moléculas; abortando.");
    return Ok(());
  }
  let mut selected: Vec<Molecule> = Vec::new();
  for tok in raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
    if let Ok(n) = tok.parse::<usize>() {
      if n >= 1 && n <= mols.len() {
        selected.push(mols[n - 1].clone());
      } else {
        println!("Índice fuera de rango: {} (ignorando)", n);
      }
    } else {
      println!("Token inválido: {} (ignorando)", tok);
    }
  }
  if selected.is_empty() {
    println!("No hay moléculas válidas seleccionadas; abortando.");
    return Ok(());
  }
  let name = prompt("Nombre de la familia (opcional): ")?;
  let desc = prompt("Descripción (opcional): ")?;
  let provenance = json!({"created_by": "cadma_example", "name": name.clone(), "description": desc.clone()});
  let mut family = MoleculeFamily::new(selected, provenance)?;
  if !name.trim().is_empty() {
    family = family.with_name(name);
  }
  if !desc.trim().is_empty() {
    family = family.with_description(desc);
  }
  match repo.save_family(family) {
    Ok(id) => {
      println!("Familia creada y guardada con id: {}", id);
      Ok(())
    }
    Err(e) => Err(Box::new(e)),
  }
}

fn save_snapshot(engine: &CadmaFlow) {
  match engine.save_snapshot() {
    Ok(_) => println!("Snapshot guardado (best-effort)."),
    Err(e) => println!("Error guardando snapshot: {}", e),
  }
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
    println!("7) Crear rama desde cursor especificado");
    println!("7) Crear rama desde cursor especificado");
    println!("8) Dump flow_data (registros persistidos)");
    println!("9) Listar familias (dominio)");
    println!("a) Crear molécula (persistir en dominio)");
    println!("b) Crear familia desde moléculas existentes en dominio");
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
        if let Some(engine) = &maybe_engine {
          if let Err(e) = create_branch_from_engine(engine) {
            println!("Error creando rama: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "8" => {
        if let Some(engine) = &maybe_engine {
          if let Err(e) = dump_flow_data(engine) {
            println!("Error volcando flow_data: {}", e);
          }
        } else {
          println!("Carga o crea un flow primero.");
        }
      }
      "9" => {
        if let Err(e) = list_families() {
          println!("Error listando familias: {}", e);
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
