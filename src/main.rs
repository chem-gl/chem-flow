use chem_persistence::DieselFlowRepository;
use chrono::Utc;
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use serde_json::json;
use std::error::Error;
use std::io::{self, Write};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn Error>> {
    let repo = chem_persistence::new_from_env().map_err(|e| Box::new(e) as Box<dyn Error>)?;

    loop {
        println!("\n== Flow CLI menu ==");
        println!("1) Ver flujos (tabla con id y parent)");
        println!("2) Crear flow");
        println!("3) Crear branch (elige parent y cursor)");
        println!("4) Eliminar flow (y subramas)");
        println!("5) Crear paso (append) en un flow");
        println!("6) Ver flujo completo (con pasos)");
        println!("7) Mostrar mapa simple de flujos");
        println!("9) Ver/actualizar status de un flow");
        println!("8) Salir");
        print!("Elige una opción: ");
        io::stdout().flush().ok();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        match choice.trim() {
            "1" => print_flows_table(&repo),
            "2" => create_flow_interactive(&repo)?,
            "3" => create_branch_interactive(&repo)?,
            "4" => delete_flow_interactive(&repo)?,
            "5" => create_step_interactive(&repo)?,
            "6" => view_flow_full(&repo)?,
            "7" => print_flow_map(&repo)?,
            "9" => view_update_status_interactive(&repo)?,
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

fn print_flows_table(repo: &DieselFlowRepository) {
    match repo.dump_tables_for_debug() {
        Ok((flows, _)) => {
            println!("\nID                                   | PARENT                                 | NAME");
            println!("-----------------------------------------------------------------------------------");
            for f in flows {
                let pid = f.parent_flow_id.map(|u| u.to_string()).unwrap_or_else(|| "-".into());
                let name = f.name.unwrap_or_else(|| "<no-name>".into());
                println!("{} | {} | {}", f.id, pid, name);
            }
        }
        Err(e) => eprintln!("Error leyendo flujos: {}", e),
    }
}

fn create_flow_interactive(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
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

fn create_branch_interactive(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, data) = repo.dump_tables_for_debug().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona parent por número:");
    for (i, f) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, f.id, f.name);
    }
    let sel = prompt("Número del parent: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let parent = &flows[idx];
    // show steps
    let items: Vec<FlowData> = data.into_iter().filter(|d| d.flow_id == parent.id).collect();
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
    match repo.create_branch(&parent.id, name_opt, status_opt, parent_cursor, json!({})) {
        Ok(id) => println!("Branch creado: {}", id),
        Err(e) => eprintln!("Error creando branch: {}", e),
    }
    Ok(())
}

fn delete_flow_interactive(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
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

fn create_step_interactive(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
    // List flows so the user can choose by number (ergonomic)
    let (flows, _data) = repo.dump_tables_for_debug().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para crear el paso:");
    for (i, f) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, f.id, f.name);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].id;
    // Append-only: place the new step at the end of the flow
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

fn view_flow_full(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
    // List flows and let the user pick one by number (ergonomic)
    let (flows, _data) = repo.dump_tables_for_debug().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para ver completo:");
    for (i, f) in flows.iter().enumerate() {
        println!("[{}] {} name={:?}", i, f.id, f.name);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].id;
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

fn print_flow_map(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
    let (flows, _) = repo.dump_tables_for_debug().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    println!("Mapa simple de flujos (parent -> child):");
    for f in &flows {
        let pid = f.parent_flow_id.map(|u| u.to_string()).unwrap_or_else(|| "-".into());
        println!("{} -> {}", pid, f.id);
    }
    Ok(())
}

fn view_update_status_interactive(repo: &DieselFlowRepository) -> Result<(), Box<dyn Error>> {
    // list flows
    let (flows, _data) = repo.dump_tables_for_debug().map_err(|e| Box::new(e) as Box<dyn Error>)?;
    if flows.is_empty() {
        println!("No hay flujos disponibles");
        return Ok(());
    }
    println!("Selecciona flow por número para ver/actualizar status:");
    for (i, f) in flows.iter().enumerate() {
        println!("[{}] {} name={:?} status={:?}", i, f.id, f.name, f.status);
    }
    let sel = prompt("Número del flow: ")?;
    let idx: usize = sel.trim().parse().map_err(|_| "Índice inválido")?;
    if idx >= flows.len() {
        println!("Índice fuera de rango");
        return Ok(());
    }
    let fid = flows[idx].id;
    // get current status via repo
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
