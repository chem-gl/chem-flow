use std::error::Error;
use std::io::{self, Write};
use serde_json::json;
use uuid::Uuid;
use flow::repository::FlowRepository;

/// Pequeño menú interactivo para administrar flujos (flows) usando el
/// repositorio proporcionado por `chem-persistence`.
///
/// Opciones soportadas:
/// 1) Ver flujos (tabla con id y parent)
/// 2) Crear flow
/// 3) Crear branch desde un flow existente
/// 4) Eliminar flow (y sus subramas)
/// 5) Salir
fn main() -> Result<(), Box<dyn Error>> {
    // Inicializar repo (aplica migraciones embebidas si procede)
    let repo = chem_persistence::new_from_env().map_err(|e| Box::new(e) as Box<dyn Error>)?;

    loop {
        println!("\n== Flow CLI menu ==");
        println!("1) Ver flujos (tabla con id y parent)");
        println!("2) Crear flow");
        println!("3) Crear branch a partir de un flow existente");
    println!("4) Eliminar flow");
    println!("5) Crear paso (append) en un flow");
    println!("6) Salir");
        print!("Elige una opción: ");
        io::stdout().flush().ok();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        match choice.trim() {
            "1" => {
                match repo.dump_tables_for_debug() {
                    Ok((flows, _data)) => {
                        println!("\nID                                   | PARENT                                 | NAME");
                        println!("-----------------------------------------------------------------------------------");
                        for f in flows {
                            let pid = f.parent_flow_id.map(|u| u.to_string()).unwrap_or_else(|| "-".into());
                            let name = f.name.unwrap_or_else(|| "<no-name>".into());
                            println!("{} | {} | {}", f.id, pid, name);
                        }
                    }
                    Err(e) => eprintln!("Error listando flujos: {}", e),
                }
            }
            "2" => {
                let name = prompt("Nombre (enter para vacío): ")?;
                let status = prompt("Estado (enter para vacío): ")?;
                let name_opt = if name.trim().is_empty() { None } else { Some(name.trim().to_string()) };
                let status_opt = if status.trim().is_empty() { None } else { Some(status.trim().to_string()) };
                match repo.create_flow(name_opt, status_opt, json!({})) {
                    Ok(id) => println!("Flow creado: {}", id),
                    Err(e) => eprintln!("Error creando flow: {}", e),
                }
            }
            "3" => {
                let parent = prompt("Parent flow id (UUID): ")?;
                let parent_id = match Uuid::parse_str(parent.trim()) {
                    Ok(u) => u,
                    Err(_) => { eprintln!("UUID inválido"); continue; }
                };
                let cursor_s = prompt("Parent cursor (número entero): ")?;
                let parent_cursor: i64 = match cursor_s.trim().parse() {
                    Ok(n) => n,
                    Err(_) => { eprintln!("Cursor inválido"); continue; }
                };
                let name = prompt("Nombre de la rama (enter para vacío): ")?;
                let status = prompt("Estado de la rama (enter para vacío): ")?;
                let name_opt = if name.trim().is_empty() { None } else { Some(name.trim().to_string()) };
                let status_opt = if status.trim().is_empty() { None } else { Some(status.trim().to_string()) };
                match repo.create_branch(&parent_id, name_opt, status_opt, parent_cursor, json!({})) {
                    Ok(id) => println!("Branch creado: {}", id),
                    Err(e) => eprintln!("Error creando branch: {}", e),
                }
            }
            "4" => {
                let id_s = prompt("Flow id a eliminar (UUID): ")?;
                let id = match Uuid::parse_str(id_s.trim()) {
                    Ok(u) => u,
                    Err(_) => { eprintln!("UUID inválido"); continue; }
                };
                let confirm = prompt(&format!("Confirma borrado de {}? escribir 'yes' para confirmar: ", id))?;
                if confirm.trim().to_lowercase() == "yes" {
                    match repo.delete_branch(&id) {
                        Ok(()) => println!("Flow eliminado: {}", id),
                        Err(e) => eprintln!("Error eliminando flow: {}", e),
                    }
                } else {
                    println!("Borrado cancelado");
                }
            }
            "5" => {
                // Crear paso
                let fid_s = prompt("Flow id (UUID) donde crear el paso: ")?;
                let fid = match Uuid::parse_str(fid_s.trim()) {
                    Ok(u) => u,
                    Err(_) => { eprintln!("UUID inválido"); continue; }
                };
                // Obtener metadata del flow para expected_version
                let meta = match repo.get_flow_meta(&fid) {
                    Ok(m) => m,
                    Err(e) => { eprintln!("No se pudo obtener meta del flow: {}", e); continue; }
                };
                let expected = meta.current_version;
                let cursor_s = prompt("Cursor para el nuevo paso (número entero): ")?;
                let cursor: i64 = match cursor_s.trim().parse() {
                    Ok(n) => n,
                    Err(_) => { eprintln!("Cursor inválido"); continue; }
                };
                let key = prompt("Key del paso (ej: step-result): ")?;
                let payload_s = prompt("Payload (JSON o texto simple): ")?;
                let metadata_s = prompt("Metadata (JSON o texto simple): ")?;
                let cmd_id_s = prompt("Command id (UUID) opcional, enter para ninguno: ")?;
                let cmd_id = if cmd_id_s.trim().is_empty() { None } else { match Uuid::parse_str(cmd_id_s.trim()) { Ok(u) => Some(u), Err(_) => { eprintln!("command_id inválido"); continue; } } };
                // Construir FlowData minimal
                let fd = flow::FlowData { id: Uuid::new_v4(), flow_id: fid, cursor, key: key.trim().to_string(), payload: serde_json::from_str(&payload_s).unwrap_or(serde_json::json!(payload_s.trim())), metadata: serde_json::from_str(&metadata_s).unwrap_or(serde_json::json!(metadata_s.trim())), command_id: cmd_id, created_at: chrono::Utc::now() };
                match repo.persist_data(&fd, expected) {
                    Ok(res) => println!("Paso creado: {:?}", res),
                    Err(e) => eprintln!("Error al persistir paso: {}", e),
                }
            }
            "6" => {
                println!("Saliendo...");
                break;
            }
            other => {
                println!("Opción inválida: {}", other);
            }
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
