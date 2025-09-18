use chrono::Utc;
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use serde_json::json;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize concrete repo and wrap in a trait object so main only
    // depends on the `FlowRepository` contract.
    let concrete = chem_persistence::new_from_env().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    let repo: Arc<dyn FlowRepository> = Arc::new(concrete);

    run_cli(repo)
}

fn print_menu() {
    println!("\n== Flow CLI menu ==");
    println!("1) Ver flujos (tabla con id y parent)");
    println!("2) Crear flow");
    println!("3) Crear branch (elige parent y cursor)");
    println!("4) Eliminar flow (y subramas)");
    println!("5) Crear paso (append) en un flow");
    println!("6) Ver flujo completo (con pasos)");
    println!("7) Mostrar mapa simple de flujos");
    println!("9) Ver/actualizar status de un flow");
    println!("10) Eliminar pasos a partir de un cursor (en una rama)");
    println!("8) Salir");
}

fn run_cli(repo: Arc<dyn FlowRepository>) -> Result<(), Box<dyn Error>> {
    loop {
        print_menu();
        print!("Elige una opción: ");
        io::stdout().flush().ok();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        match choice.trim() {
            "1" => show_flows_interactive(repo.as_ref())?,
            "2" => create_flow_interactive(repo.as_ref())?,
            "3" => create_branch_interactive(repo.as_ref())?,
            "4" => delete_flow_interactive(repo.as_ref())?,
            "5" => create_step_interactive(repo.as_ref())?,
            "6" => view_flow_full(repo.as_ref())?,
            "7" => print_flow_map(repo.as_ref())?,
            "10" => delete_steps_from_cursor_interactive(repo.as_ref())?,
            "9" => view_update_status_interactive(repo.as_ref())?,
            "8" => {
                println!("Saliendo...");
                break;
            }
            other => println!("Opción inválida: {}", other),
        }
    }
    Ok(())
}

fn prompt(msg: &str) -> io::Result<String> {
    print!("{}", msg);
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    Ok(s)
}

// ...existing code reused from src/example-main.rs ...

type FlowEntry = (Uuid, Option<Uuid>, Option<String>, Option<String>);

fn get_flows_and_data(repo: &dyn FlowRepository) -> Result<(Vec<FlowEntry>, Vec<FlowData>), Box<dyn Error>> {
    let (flows, data) = repo.dump_tables_for_debug().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    let mapped: Vec<FlowEntry> = flows.into_iter()
                                      .map(|f| (f.id, f.parent_flow_id, f.name, f.status))
                                      .collect();
    Ok((mapped, data))
}

fn show_flows_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    match repo.list_flow_ids() {
        Ok(ids) => {
            println!("Flujos ({}):", ids.len());
            if ids.is_empty() {
                println!("(no hay registros)");
                return Ok(());
            }
            let (flows, _data) = get_flows_and_data(repo)?;
            println!("{:<40} {:<40} {:<20} {:<10}", "id", "parent", "name", "status");
            for (id, parent, name, status) in flows {
                let pid = parent.map(|u| u.to_string()).unwrap_or_else(|| "-".into());
                let name_s = name.unwrap_or_else(|| "-".into());
                let status_s = status.unwrap_or_else(|| "-".into());
                println!("{:<40} {:<40} {:<20} {:<10}", id, pid, name_s, status_s);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error listando flujos: {}", e);
            Err(Box::new(e) as Box<dyn Error>)
        }
    }
}

fn create_flow_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let name = prompt("Nombre (enter para vacío): ")?;
    let status = prompt("Estado (enter para vacío): ")?;
    let name_opt = if name.trim().is_empty() {
        None
    } else {
        Some(name.trim().to_string())
    };
    let status_opt = if status.trim().is_empty() {
        None
    } else {
        Some(status.trim().to_string())
    };
    match repo.create_flow(name_opt, status_opt, json!({})) {
        Ok(id) => println!("Flow creado: {}", id),
        Err(e) => eprintln!("Error creando flow: {}", e),
    }
    Ok(())
}

fn create_branch_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, data) = get_flows_and_data(repo)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona parent por número:");
    for (i, (id, _parent, name, _status)) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, id, name);
    }
    let sel = prompt("Número del parent: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let parent_id = flows[idx].0;
    let items: Vec<FlowData> = data.into_iter().filter(|d| d.flow_id == parent_id).collect();
    if items.is_empty() {
        println!("El flow no tiene pasos aún. El cursor por defecto será 0");
    }
    println!("Pasos (cursor):");
    for it in &items {
        println!("- {}: {}", it.cursor, it.key);
    }
    let cursor_s = prompt("Cursor desde el que crear la rama (número entero): ")?;
    let parent_cursor: i64 = cursor_s.trim().parse().map_err(|_| "Cursor inválido")?;
    let name = prompt("Nombre de la rama (enter para vacío): ")?;
    let status = prompt("Estado de la rama (enter para vacío): ")?;
    let name_opt = if name.trim().is_empty() {
        None
    } else {
        Some(name.trim().to_string())
    };
    let status_opt = if status.trim().is_empty() {
        None
    } else {
        Some(status.trim().to_string())
    };
    match repo.create_branch(&parent_id, name_opt, status_opt, parent_cursor, json!({})) {
        Ok(id) => println!("Branch creado: {}", id),
        Err(e) => eprintln!("Error creando branch: {}", e),
    }
    Ok(())
}

fn delete_flow_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let id_s = prompt("Flow id a eliminar (UUID): ")?;
    let id = Uuid::parse_str(id_s.trim()).map_err(|_| "UUID inválido")?;
    let confirm = prompt(&format!("Confirma borrado de {}? escribir 'yes' para confirmar: ", id))?;
    if confirm.trim().to_lowercase() == "yes" {
        match repo.delete_branch(&id) {
            Ok(()) => println!("Flow eliminado: {}", id),
            Err(e) => eprintln!("Error eliminando flow: {}", e),
        }
    } else {
        println!("Borrado cancelado");
    }
    Ok(())
}

fn create_step_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, _data) = get_flows_and_data(repo)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para crear el paso:");
    for (i, (id, _parent, name, _status)) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, id, name);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].0;
    let meta = repo.get_flow_meta(&fid).map_err(|e| Box::new(e) as Box<dyn Error>)?;
    let expected = meta.current_version;
    let cursor: i64 = meta.current_cursor + 1;
    println!("El nuevo paso se añadirá al final del flujo en cursor={} (append-only)",
             cursor);
    let key = prompt("Key del paso (ej: step-result): ")?;
    let payload_s = prompt("Payload (JSON o texto simple): ")?;
    let metadata_s = prompt("Metadata (JSON o texto simple): ")?;
    let cmd_id_s = prompt("Command id (UUID) opcional, enter para ninguno: ")?;
    let cmd_id = if cmd_id_s.trim().is_empty() {
        None
    } else {
        Some(Uuid::parse_str(cmd_id_s.trim()).map_err(|_| "command_id inválido")?)
    };
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: fid,
                        cursor,
                        key: key.trim().to_string(),
                        payload: serde_json::from_str(&payload_s).unwrap_or(serde_json::json!(payload_s.trim())),
                        metadata: serde_json::from_str(&metadata_s).unwrap_or(serde_json::json!(metadata_s.trim())),
                        command_id: cmd_id,
                        created_at: Utc::now() };
    match repo.persist_data(&fd, expected) {
        Ok(res) => println!("Paso creado: {:?}", res),
        Err(e) => eprintln!("Error al persistir paso: {}", e),
    }
    Ok(())
}

fn view_flow_full(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, _data) = get_flows_and_data(repo)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para ver completo:");
    for (i, (id, _parent, name, _status)) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, id, name);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].0;
    let meta = repo.get_flow_meta(&fid).map_err(|e| Box::new(e) as Box<dyn Error>)?;
    println!("Flow {} name={:?} parent={:?} cursor={} version={}",
             meta.id, meta.name, meta.parent_flow_id, meta.current_cursor, meta.current_version);
    let items = repo.read_data(&fid, 0).map_err(|e| Box::new(e) as Box<dyn Error>)?;
    println!("--- pasos (cursor | key | payload) ---");
    for it in items {
        println!("{} | {} | {}", it.cursor, it.key, it.payload);
    }
    Ok(())
}

fn print_flow_map(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, _) = get_flows_and_data(repo)?;
    println!("Mapa simple de flujos (parent -> child):");
    for (id, parent, _name, _status) in &flows {
        let pid = parent.map(|u| u.to_string()).unwrap_or_else(|| "-".into());
        println!("{} -> {}", pid, id);
    }
    Ok(())
}

fn view_update_status_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, _data) = get_flows_and_data(repo)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para ver/actualizar status:");
    for (i, (id, _parent, name, status)) in flows.iter().enumerate() {
        println!("[{}] {} name={:?} status={:?}", i, id, name, status);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].0;
    match repo.get_flow_status(&fid) {
        Ok(cur) => println!("Status actual: {:?}", cur),
        Err(e) => eprintln!("Error obteniendo status: {}", e),
    }
    let new = prompt("Nuevo status (enter para dejar vacío, o escribir 'skip' para no cambiar): ")?;
    if new.trim().to_lowercase() == "skip" {
        println!("No se realizó ningún cambio");
        return Ok(());
    }
    let new_status = if new.trim().is_empty() {
        None
    } else {
        Some(new.trim().to_string())
    };
    match repo.set_flow_status(&fid, new_status) {
        Ok(meta) => println!("Status actualizado. Nuevo meta: id={} status={:?}", meta.id, meta.status),
        Err(e) => eprintln!("Error actualizando status: {}", e),
    }
    Ok(())
}

fn delete_steps_from_cursor_interactive(repo: &dyn FlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, data) = get_flows_and_data(repo)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para eliminar pasos a partir de un cursor:");
    for (i, (id, _parent, name, _status)) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, id, name);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].0;
    let items: Vec<FlowData> = data.into_iter().filter(|d| d.flow_id == fid).collect();
    if items.is_empty() {
        println!("El flow no tiene pasos aún. No hay nada que eliminar.");
        return Ok(());
    }
    println!("Pasos actuales (cursor | key):");
    for it in &items {
        println!("- {} | {}", it.cursor, it.key);
    }
    let cursor_s = prompt("Eliminar a partir de qué cursor? (entero, inclusive; puedes elegir 1): ")?;
    let from_cursor: i64 = cursor_s.trim().parse().map_err(|_| "Cursor inválido")?;
    if from_cursor < 1 {
        println!("El cursor debe ser >= 1");
        return Ok(());
    }
    match repo.delete_from_step(&fid, from_cursor) {
        Ok(()) => println!("Pasos desde cursor {} eliminados (inclusive) en flow {}", from_cursor, fid),
        Err(e) => eprintln!("Error eliminando pasos: {}", e),
    }
    Ok(())
}
