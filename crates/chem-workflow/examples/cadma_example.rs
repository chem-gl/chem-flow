// cadma_example.rs
//
// Ejemplo interactivo que muestra la creación, ejecución y
// persistencia de un `CadmaFlow` usando los repositorios de
// persistencia del workspace (chem-persistence).
use chem_domain::DomainRepository;
use chem_persistence::{new_domain_from_env, new_flow_from_env};
use chem_workflow::step::StepInfo;
use chem_workflow::{
  factory::ChemicalWorkflowFactory,
  flows::{cadma_flow::steps::step2::Step2Input, CadmaFlow},
  ChemicalFlowEngine,
};
use flow::repository::FlowRepository;
use serde_json::json;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use uuid::Uuid;

// Helper to get the flow name from the repository, or a default if not present
fn get_flow_name(repo: &dyn FlowRepository, id: &Uuid) -> String {
  repo.get_flow_meta(id).ok().and_then(|meta| meta.name).unwrap_or_else(|| "sin nombre".to_string())
}

// ========== HELPERS ==========
fn prompt(msg: &str) -> io::Result<String> {
  print!("{}", msg);
  io::stdout().flush()?;
  let mut s = String::new();
  io::stdin().read_line(&mut s)?;
  Ok(s.trim_end().to_string())
}
fn print_menu() {
  println!("\n== CadmaFlow Demo (Base de Datos) ==");
  println!("1) Crear nuevo flow");
  println!("2) Listar flujos existentes");
  println!("3) Ejecutar flujo interactivo");
  println!("4) Crear rama desde flow existente");
  println!("5) Ver pasos de un flow");
  println!("6) Dump completo de todos los flujos");
  println!("7) Crear familia desde SMILES (dominio)");
  println!("8) Listar familias existentes con sus moléculas");
  println!("q) Salir");
}

fn select_flow_from_list(repo: &dyn FlowRepository, prompt_msg: &str) -> Result<Option<Uuid>, Box<dyn Error>> {
  let ids = repo.list_flow_ids()?;
  if ids.is_empty() {
    println!("No hay flujos disponibles");
    return Ok(None);
  }
  println!("{}", prompt_msg);
  for (i, id) in ids.iter().enumerate() {
    let name = get_flow_name(repo, id);
    println!("[{}] {} - {}", i, id, name);
  }
  let input = prompt("Selecciona un índice: ")?;
  let idx: usize = input.trim().parse().map_err(|_| "Índice inválido")?;
  if idx >= ids.len() {
    println!("Índice fuera de rango");
    return Ok(None);
  }
  Ok(Some(ids[idx]))
}
// ========== FLOW OPERATIONS ==========
fn create_flow() -> Result<(), Box<dyn Error>> {
  let name = prompt("Nombre del flow (enter para nombre por defecto): ")?;
  let flow_name = if name.is_empty() { "cadma-flow".to_string() } else { name };
  match ChemicalWorkflowFactory::create::<CadmaFlow>(flow_name) {
    Ok(engine_box) => println!("✅ Flow creado exitosamente: {}", (*engine_box).id()),
    Err(e) => eprintln!("❌ Error creando flow: {}", e),
  }
  Ok(())
}
fn list_flows() -> Result<(), Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  let ids = repo.list_flow_ids()?;
  println!("\n📋 Flujos encontrados ({}):", ids.len());
  for id in ids {
    let name = get_flow_name(&repo, &id);
    println!("  • {} - {}", id, name);
  }
  Ok(())
}
fn execute_step_interactive(engine: &mut CadmaFlow) -> Result<StepInfo, Box<dyn Error>> {
  let step_name = engine.current_step_name()?;
  println!("▶️  Ejecutando paso: {}", step_name);
  // Antes de ejecutar, comprobar si ya existe payload para el paso.
  // En lugar de devolver un error fatal, informamos y devolvemos un
  // StepInfo especial para que la UI pueda regresar al menú de forma
  // amigable.
  if let Ok(Some(_)) = engine.get_last_step_payload(&step_name) {
    println!("ℹ️  El paso '{}' ya fue ejecutado para este flow; no hay acciones pendientes.",
             step_name);
    return Ok(StepInfo { payload: serde_json::json!({"status": "already_executed", "step": step_name}),
                         metadata: serde_json::json!({}) });
  }

  let result = if step_name.to_lowercase() == "step2" {
    let multiplier = prompt("Ingrese multiplicador (entero): ")?.trim().parse().unwrap_or(1);
    let input = Step2Input { multiplier };
    engine.execute_current_step_typed(&input)?
  } else if step_name.to_lowercase().contains("family_reference_step1") || step_name.to_lowercase().contains("step1") {
    // Ofrecemos un pequeño sub-menú para cubrir todas las formas de usar el
    // Step1: 1) ingresar SMILES, 2) seleccionar familias existentes, 3)
    // combinar ambas. También listamos moléculas disponibles en el repo.
    println!("\n--- Opciones para Step1 (Family Reference) ---");
    println!("1) Ingresar SMILES separados por comas");
    println!("2) Seleccionar una o más familias existentes en el repositorio");
    println!("3) Combinar SMILES + seleccionar familias");
    println!("4) Cancelar");
    let choice = prompt("Selecciona una opción: ")?;
    if choice.trim() == "4" {
      println!("Operación cancelada");
      engine.execute_current_step(&json!({}))?
    } else {
      // Prepara contenedores
      let mut families_opt: Option<Vec<Uuid>> = None;
      let mut mols_opt: Option<Vec<chem_domain::Molecule>> = None;

      if choice.trim() == "2" || choice.trim() == "3" {
        // Listar familias disponibles desde el domain_repo y permitir selección
        match engine.domain_repo.list_families() {
          Ok(listed) => {
            if listed.is_empty() {
              println!("No hay familias en el repositorio");
            } else {
              println!("Familias disponibles:");
              for (i, f) in listed.iter().enumerate() {
                // Build a short comma-separated list of InChIKeys for display
                let mols_list = f.molecules().iter().map(|m| m.inchikey().to_string()).collect::<Vec<_>>().join(", ");
                let name = f.name().map(|s| s.as_str()).unwrap_or("sin nombre");
                println!("  [{}] {} - {} ({})", i, f.id(), name, mols_list);
              }
              let sel = prompt("Indices de familias separados por comas (enter para omitir): ")?;
              if !sel.trim().is_empty() {
                let mut sel_ids = Vec::new();
                for s in sel.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                  if let Ok(idx) = s.parse::<usize>() {
                    if idx < listed.len() {
                      sel_ids.push(listed[idx].id());
                    } else {
                      println!("Índice fuera de rango: {}", idx);
                    }
                  } else {
                    println!("Índice inválido ignorado: {}", s);
                  }
                }
                if !sel_ids.is_empty() {
                  families_opt = Some(sel_ids);
                }
              }
            }
          }
          Err(e) => println!("Error listando familias: {}", e),
        }
      }

      if choice.trim() == "1" || choice.trim() == "3" {
        // Pedir SMILES y convertir a Molecule
        let smiles_raw = prompt("Ingrese SMILES separados por comas (enter para omitir): ")?;
        if !smiles_raw.trim().is_empty() {
          let mut mv = Vec::new();
          for s in smiles_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match chem_domain::Molecule::from_smiles(s) {
              Ok(m) => mv.push(m),
              Err(e) => println!("Error creando molécula para SMILES '{}': {}", s, e),
            }
          }
          if !mv.is_empty() {
            mols_opt = Some(mv);
          }
        }
      }

      // Nombre y descripción opcionales.
      // Si el usuario seleccionó sólo una familia existente y no proporcionó
      // moléculas explícitas, no preguntamos la descripción por defecto
      // (usaremos la descripción de la familia seleccionada salvo que el
      // usuario decida cambiarla al introducir un nuevo nombre).
      let new_name: String;
      let new_desc: String;
      let single_existing_family = families_opt.as_ref().map(|v| v.len() == 1).unwrap_or(false);
      let has_explicit_mols = mols_opt.is_some();

      if single_existing_family && !has_explicit_mols {
        // Sólo pedir nombre (opcional). Si el usuario deja el nombre vacío
        // se usará la familia existente sin crear una nueva versión.
        let tmp_name = prompt("Nombre de la nueva familia (opcional, enter para usar la existente): ")?;
        if !tmp_name.trim().is_empty() {
          // Si se provee un nuevo nombre, preguntar descripción opcional
          let tmp_desc = prompt("Descripción de la nueva familia (opcional): ")?;
          new_name = tmp_name;
          new_desc = tmp_desc;
        } else {
          new_name = tmp_name;
          new_desc = String::new();
        }
      } else {
        // En otros casos (múltiples familias seleccionadas o moléculas explícitas)
        // pedir nombre y descripción como antes.
        new_name = prompt("Nombre de la nueva familia (opcional): ")?;
        new_desc = prompt("Descripción de la nueva familia (opcional): ")?;
      }

      let input = json!({
        "families": families_opt,
        "molecules": mols_opt,
        "new_family_name": if new_name.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(new_name) },
        "new_family_description": if new_desc.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(new_desc) },
      });
      engine.execute_current_step(&input)?
    }
  } else {
    engine.execute_current_step(&json!({}))?
  };
  println!("📊 Resultado: {}", result.payload);
  Ok(result)
}
fn persist_step_result(engine: &CadmaFlow, info: StepInfo) -> Result<(), Box<dyn Error>> {
  let cmd_id_input = prompt("Command ID (UUID opcional, enter para omitir): ")?;
  let command_id = if cmd_id_input.trim().is_empty() { None } else { Some(Uuid::parse_str(&cmd_id_input)?) };
  match engine.persist_step_result(&engine.current_step_name()?, info, -1, command_id) {
    Ok(res) => println!("💾 Persistido: {:?}", res),
    Err(e) => eprintln!("❌ Error al persistir: {}", e),
  }
  Ok(())
}
fn run_flow_interactive(engine: &mut CadmaFlow) -> Result<(), Box<dyn Error>> {
  println!("\n🔧 Flow seleccionado: {}", engine.id());
  println!("   Paso actual: {}, Estado: {:?}", engine.current_step(), engine.status());
  loop {
    if engine.current_step_name().is_err() {
      println!("🎉 El flujo ha finalizado");
      break;
    }
    println!("\nOpciones:");
    println!("  r) Ejecutar siguiente paso");
    println!("  s) Mostrar último payload");
    println!("  b) Volver al menú principal");
    match prompt("Selecciona una opción: ")?.trim() {
      "r" => {
        let result = execute_step_interactive(engine)?;
        // Si el resultado indica que el paso ya fue ejecutado, no
        // preguntar por persistir ni avanzar: solo volver al menú.
        let already = result.payload.get("status").and_then(|v| v.as_str()) == Some("already_executed");
        if already {
          println!("✅ El flujo ya tenía resultado para este paso. Volviendo al menú.");
          continue;
        }

        if prompt("¿Persistir resultado? (y/N): ")?.to_lowercase().starts_with('y') {
          persist_step_result(engine, result)?;
        }
        if prompt("¿Avanzar al siguiente paso? (y/N): ")?.to_lowercase().starts_with('y') {
          let _ = engine.advance_step();
          println!("➡️  Avanzado al paso {}", engine.current_step());
        }
      }
      "s" => {
        let name = engine.current_step_name()?;
        match engine.get_last_step_payload(&name) {
          Ok(Some(payload)) => println!("📄 Último payload: {}", payload),
          Ok(None) => println!("ℹ️  No hay payload para {}", name),
          Err(e) => eprintln!("❌ Error leyendo payload: {}", e),
        }
      }
      "b" => break,
      other => println!("❌ Opción desconocida: {}", other),
    }
  }
  Ok(())
}
fn select_and_run_flow() -> Result<(), Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  if let Some(flow_id) = select_flow_from_list(&repo, "Selecciona un flow para ejecutar:")? {
    let mut engine =
      ChemicalWorkflowFactory::load::<CadmaFlow>(&flow_id).map_err(|e| format!("Error cargando engine: {}", e))?;
    run_flow_interactive(&mut engine)?;
  }
  Ok(())
}
fn create_branch() -> Result<(), Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  if let Some(parent_id) = select_flow_from_list(&repo, "Selecciona flow padre:")? {
    let cursor_input = prompt("Cursor desde el que crear la rama: ")?;
    let parent_cursor: i64 = cursor_input.trim().parse()?;

    match repo.create_branch(&parent_id, parent_cursor, json!({})) {
      Ok(branch_id) => println!("🌿 Rama creada exitosamente: {}", branch_id),
      Err(e) => eprintln!("❌ Error creando rama: {}", e),
    }
  }
  Ok(())
}
fn view_flow_steps() -> Result<(), Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  if let Some(flow_id) = select_flow_from_list(&repo, "Selecciona flow para ver pasos:")? {
    let meta = repo.get_flow_meta(&flow_id)?;
    let steps = repo.read_data(&flow_id, 0)?;
    println!("\n📊 Flow: {} (Cursor: {}, Versión: {})",
             meta.name.unwrap_or_else(|| "sin nombre".to_string()),
             meta.current_cursor,
             meta.current_version);
    println!("┌{:─<20}┬{:─<30}┬{:─<50}┐", "", "", "");
    println!("│ {:<18} │ {:<28} │ {:<48} │", "Cursor", "Key", "Payload");
    println!("├{:─<20}┼{:─<30}┼{:─<50}┤", "", "", "");
    for step in steps {
      println!("│ {:<18} │ {:<28} │ {:<48} │", step.cursor, step.key, step.payload);
    }
    println!("└{:─<20}┴{:─<30}┴{:─<50}┘", "", "", "");
  }
  Ok(())
}
fn dump_all_flows() -> Result<(), Box<dyn Error>> {
  let repo = new_flow_from_env()?;
  let (metas, datas) = repo.dump_tables_for_debug()?;
  println!("\n📋 DUMP COMPLETO - {} flujos, {} registros", metas.len(), datas.len());
  for meta in metas {
    println!("\n🔧 Flow: {} (Cursor: {}, Versión: {})",
             meta.id, meta.current_cursor, meta.current_version);
    let flow_data: Vec<_> = datas.iter().filter(|d| d.flow_id == meta.id).collect();
    for data in flow_data {
      println!("   ├─ {} | {} | {}", data.cursor, data.key, data.payload);
    }
  }
  Ok(())
}

// ===== Domain helpers for families (outside flow) =====
fn list_families_with_molecules() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  match repo.list_families() {
    Ok(fams) => {
      println!("\n📋 Familias encontradas: {}", fams.len());
      for f in fams {
        println!("- Family id={} name={:?} size={}", f.id(), f.name(), f.len());
        for m in f.molecules() {
          println!("    - InChIKey={} SMILES={}", m.inchikey(), m.smiles());
        }
      }
    }
    Err(e) => println!("Error listando familias: {}", e),
  }
  Ok(())
}

fn create_family_from_smiles_interactive() -> Result<(), Box<dyn Error>> {
  let repo = new_domain_from_env()?;
  let smiles_raw = prompt("Ingrese SMILES separados por comas: ")?;
  if smiles_raw.trim().is_empty() {
    println!("No se ingresaron SMILES");
    return Ok(());
  }
  let mut mols = Vec::new();
  for s in smiles_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
    match chem_domain::Molecule::from_smiles(s) {
      Ok(m) => mols.push(m),
      Err(e) => println!("Error creando molécula para SMILES '{}': {}", s, e),
    }
  }
  if mols.is_empty() {
    println!("No se pudieron crear moléculas válidas desde los SMILES provistos");
    return Ok(());
  }
  let name = prompt("Nombre de la nueva familia (opcional): ")?;
  let desc = prompt("Descripción de la nueva familia (opcional): ")?;
  let provenance = json!({ "created_by": "cadma_example_create_family", "timestamp": chrono::Utc::now().to_rfc3339() });
  let mut family = chem_domain::MoleculeFamily::new(mols.into_iter(), provenance)?;
  if !name.trim().is_empty() {
    family = family.with_name(name)?;
  }
  if !desc.trim().is_empty() {
    family = family.with_description(desc)?;
  }
  match repo.save_family(family) {
    Ok(id) => println!("Familia creada con id={}", id),
    Err(e) => println!("Error guardando familia: {}", e),
  }
  Ok(())
}
// ========== MAIN APPLICATION ==========
fn setup_repository() -> Result<Arc<dyn FlowRepository>, Box<dyn Error>> {
  match new_flow_from_env() {
    Ok(repo) => Ok(Arc::new(repo)),
    Err(e) => {
      let error_msg = e.to_string();
      if error_msg.contains("was compiled without 'pg' feature") {
        eprintln!("\n❌ Error de configuración:");
        eprintln!("   chem-persistence fue compilado sin soporte para PostgreSQL");
        eprintln!("   pero DATABASE_URL apunta a una base PostgreSQL.");
        eprintln!("\n💡 Soluciones:");
        eprintln!("   1) Compilar con soporte PostgreSQL:");
        eprintln!("      cargo run -p chem-workflow --example cadma_example --features pg");
        eprintln!("   2) Usar SQLite:");
        eprintln!("      export DATABASE_URL=\"file:./chemflow_demo.db\"");
      }
      Err(Box::new(e))
    }
  }
}
fn main() -> Result<(), Box<dyn Error>> {
  println!("🚀 Iniciando CadmaFlow Demo");
  // Configuración inicial
  let _repo = setup_repository()?;
  // Bucle principal de la aplicación
  loop {
    print_menu();
    match prompt("Opción: ")?.trim() {
      "1" => create_flow()?,
      "2" => list_flows()?,
      "3" => select_and_run_flow()?,
      "4" => create_branch()?,
      "5" => view_flow_steps()?,
      "6" => dump_all_flows()?,
      "7" => create_family_from_smiles_interactive()?,
      "8" => list_families_with_molecules()?,
      "q" | "Q" => {
        println!("👋 ¡Hasta pronto!");
        break;
      }
      other => println!("❌ Opción no válida: {}", other),
    }
  }
  Ok(())
}
