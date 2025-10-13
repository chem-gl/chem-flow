//! Ejemplo completo de uso del sistema de árbol de flujos
//!
//! Este ejemplo demuestra:
//! - Creación de flujos y ramas
//! - Evolución independiente
//! - Snapshots y rehidratación
//! - Eliminación de ramas
//! - Visualización del árbol

use chrono::Utc;
use flow::domain::{FlowData, PersistResult};
use flow::repository::FlowRepository;
use flow::stubs::InMemoryFlowRepository;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Helper para añadir un paso con locking optimista correcto
fn append_step(repo: &dyn FlowRepository, flow_id: &Uuid, content: &str) -> i64 {
  let meta = repo.get_flow_meta(flow_id).expect("get meta");
  let next_cursor = meta.current_cursor + 1;

  let step = FlowData { id: Uuid::new_v4(),
                        flow_id: *flow_id,
                        cursor: next_cursor,
                        key: format!("step_{}", next_cursor),
                        payload: json!({"content": content, "step": next_cursor}),
                        metadata: json!({"timestamp": Utc::now().to_rfc3339()}),
                        command_id: None,
                        created_at: Utc::now() };

  let result = repo.persist_data(&step, meta.current_version).expect("persist");
  match result {
    PersistResult::Ok { .. } => next_cursor,
    PersistResult::Conflict => panic!("Conflicto de versión"),
  }
}

/// Imprime el árbol de flujos de forma visual
fn print_tree(repo: &dyn FlowRepository, flow_id: &Uuid, indent: usize) {
  let meta = repo.get_flow_meta(flow_id).expect("get meta");
  let data = repo.read_data(flow_id, 0).expect("read data");

  let indent_str = "  ".repeat(indent);
  let flow_name = meta.name.unwrap_or_else(|| format!("Flow {}", flow_id));

  println!("{}📁 {} ({} pasos)", indent_str, flow_name, data.len());

  // Mostrar primeros y últimos 3 pasos
  let show_count = 3.min(data.len());
  for item in data.iter().take(show_count) {
    println!("{}  └─ Paso {}: {}",
             indent_str,
             item.cursor,
             item.payload["content"].as_str().unwrap_or("???"));
  }

  if data.len() > show_count * 2 {
    println!("{}  └─ ... ({} pasos más)", indent_str, data.len() - show_count * 2);
    for item in data.iter().skip(data.len() - show_count) {
      println!("{}  └─ Paso {}: {}",
               indent_str,
               item.cursor,
               item.payload["content"].as_str().unwrap_or("???"));
    }
  }

  // Buscar hijos (ramas)
  let all_ids = repo.list_flow_ids().expect("list");
  for child_id in all_ids {
    let child_meta = repo.get_flow_meta(&child_id).expect("child meta");
    if child_meta.parent_flow_id == Some(*flow_id) {
      print_tree(repo, &child_id, indent + 1);
    }
  }
}

fn main() {
  println!("🧪 Ejemplo: Sistema de Árbol de Flujos\n");
  println!("═══════════════════════════════════════\n");

  let repo = Arc::new(InMemoryFlowRepository::new());

  // ========================================
  // FASE 1: Crear flujo principal
  // ========================================
  println!("📋 FASE 1: Creando flujo principal...\n");

  let main_id = repo.create_flow(Some("Experimento Principal".into()),
                                 Some("active".into()),
                                 json!({"experiment": "synthesis-route-1", "started": Utc::now().to_rfc3339()}))
                    .expect("crear flujo principal");

  println!("✅ Flujo principal creado: {}\n", main_id);

  // Añadir 10 pasos al flujo principal
  println!("📝 Añadiendo 10 pasos al flujo principal...");
  for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("Síntesis paso {}: Preparación reactivo {}", i, i));
  }

  let main_meta = repo.get_flow_meta(&main_id).expect("meta");
  println!("✅ {} pasos añadidos (versión {})\n",
           main_meta.current_cursor, main_meta.current_version);

  // ========================================
  // FASE 2: Guardar snapshot
  // ========================================
  println!("💾 FASE 2: Guardando snapshot en paso 10...\n");

  let snap_id = repo.save_snapshot(&main_id,
                                   10,
                                   "snapshot_checkpoint_1",
                                   json!({"description": "Checkpoint después de preparación"}))
                    .expect("guardar snapshot");

  println!("✅ Snapshot guardado: {}\n", snap_id);

  // ========================================
  // FASE 3: Crear ramas para explorar alternativas
  // ========================================
  println!("🌿 FASE 3: Creando ramas para explorar alternativas...\n");

  // Rama 1: Explorar método alternativo desde paso 5
  println!("  • Creando Rama 1 (método alternativo) desde paso 5...");
  let branch1_id =
    repo.create_branch(&main_id, 5, json!({"reason": "Probar catalizador alternativo"})).expect("crear rama 1");

  // Continuar rama 1 con pasos diferentes
  for i in 6..=9 {
    append_step(&*repo, &branch1_id, &format!("Método Alt: Catálisis con Pd/C paso {}", i));
  }
  println!("    ✅ Rama 1 creada con 4 pasos nuevos (total: 9 pasos)\n");

  // Rama 2: Explorar temperatura diferente desde paso 8
  println!("  • Creando Rama 2 (temperatura alta) desde paso 8...");
  let branch2_id = repo.create_branch(&main_id, 8, json!({"reason": "Probar temperatura elevada"})).expect("crear rama 2");

  for i in 9..=11 {
    append_step(&*repo, &branch2_id, &format!("Alta Temp: {} °C paso {}", 150 + i * 10, i));
  }
  println!("    ✅ Rama 2 creada con 3 pasos nuevos (total: 11 pasos)\n");

  // ========================================
  // FASE 4: Subrama (branch de branch)
  // ========================================
  println!("🌳 FASE 4: Creando subrama desde Rama 1...\n");

  println!("  • Creando Subrama 1.1 desde paso 7 de Rama 1...");
  let subbranch_id =
    repo.create_branch(&branch1_id, 7, json!({"reason": "Variar tiempo de reacción"})).expect("crear subrama");

  append_step(&*repo, &subbranch_id, "Tiempo extendido: 12h paso 8");
  append_step(&*repo, &subbranch_id, "Análisis intermedio paso 9");
  println!("    ✅ Subrama 1.1 creada (total: 9 pasos)\n");

  // ========================================
  // FASE 5: Continuar flujo principal
  // ========================================
  println!("➡️  FASE 5: Continuando flujo principal...\n");

  for i in 11..=15 {
    append_step(&*repo, &main_id, &format!("Principal: Purificación paso {}", i));
  }

  let updated_meta = repo.get_flow_meta(&main_id).expect("meta");
  println!("✅ Flujo principal ahora tiene {} pasos\n", updated_meta.current_cursor);

  // ========================================
  // FASE 6: Visualizar árbol completo
  // ========================================
  println!("🌲 FASE 6: Árbol de flujos actual:\n");
  println!("═══════════════════════════════════════\n");

  print_tree(&*repo, &main_id, 0);

  println!();

  // ========================================
  // FASE 7: Rehidratación desde snapshot
  // ========================================
  println!("🔄 FASE 7: Simulando rehidratación...\n");

  let snap_opt = repo.load_latest_snapshot(&main_id).expect("cargar snapshot");
  if let Some(snap) = snap_opt {
    println!("  📍 Último snapshot encontrado:");
    println!("     • Cursor: {}", snap.cursor);
    println!("     • State pointer: {}", snap.state_ptr);

    // Simular replay de pasos posteriores
    let replay_data = repo.read_data(&main_id, snap.cursor).expect("replay");
    println!("     • Pasos para replay: {}", replay_data.len());
    println!("       (del paso {} al {})", snap.cursor + 1, updated_meta.current_cursor);
  }

  println!();

  // ========================================
  // FASE 8: Estadísticas finales
  // ========================================
  println!("📊 FASE 8: Estadísticas finales:\n");

  let all_ids = repo.list_flow_ids().expect("list");
  println!("  • Total de flujos: {}", all_ids.len());

  for id in &all_ids {
    let meta = repo.get_flow_meta(id).expect("meta");
    let count = repo.count_steps(id).expect("count");
    let flow_type = if meta.parent_flow_id.is_some() { "Rama" } else { "Principal" };

    println!("    - {}: {} ({} pasos, versión {})",
             flow_type,
             meta.name.unwrap_or_else(|| "Sin nombre".into()),
             count,
             meta.current_version);
  }

  println!();

  // ========================================
  // FASE 9: Eliminar una rama
  // ========================================
  println!("🗑️  FASE 9: Eliminando Rama 2 (descartando experimento)...\n");

  println!("  • Eliminando rama y sus datos...");
  repo.delete_branch(&branch2_id).expect("eliminar rama");

  println!("  ✅ Rama 2 eliminada");
  println!("  ✅ Flujo principal y otras ramas intactos\n");

  // Verificar
  assert!(!repo.branch_exists(&branch2_id).expect("check exists"));
  assert!(repo.branch_exists(&main_id).expect("check main"));
  assert!(repo.branch_exists(&branch1_id).expect("check branch1"));

  // ========================================
  // FASE 10: Árbol final
  // ========================================
  println!("🎯 FASE 10: Árbol final después de eliminación:\n");
  println!("═══════════════════════════════════════\n");

  print_tree(&*repo, &main_id, 0);

  println!();
  println!("✅ Ejemplo completado exitosamente!");
  println!("\n📝 Resumen:");
  println!("   • Flujo principal: 15 pasos");
  println!("   • Rama 1: 9 pasos (5 heredados + 4 nuevos)");
  println!("   • Subrama 1.1: 9 pasos (7 heredados + 2 nuevos)");
  println!("   • Rama 2: ❌ eliminada");
  println!("   • Snapshots: 1 guardado en paso 10");
  println!("   • Sin ciclos, sin merges, sin duplicaciones ✓");
}
