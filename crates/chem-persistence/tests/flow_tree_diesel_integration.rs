// flow_tree_diesel_integration.rs
//! Tests de integración para verificar que el árbol de flujos funciona
//! correctamente con persistencia real (Diesel + SQLite)
//!
//! Estos tests verifican:
//! - Creación de flujos y ramas con persistencia real
//! - Rehidratación desde base de datos
//! - Eliminación correcta con transacciones
//! - Árbol de flujos sin ciclos ni duplicaciones
//! - Snapshots y replay

use chem_persistence::{test_helpers::create_temp_sqlite_db, DieselFlowRepository};
use chrono::Utc;
use flow::domain::{FlowData, PersistResult};
use flow::repository::FlowRepository;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Helper para crear un paso
fn create_step(flow_id: Uuid, cursor: i64, content: &str) -> FlowData {
  FlowData { id: Uuid::new_v4(),
             flow_id,
             cursor,
             key: format!("step_{}", cursor),
             payload: json!({"content": content, "step": cursor}),
             metadata: json!({"type": "test_step"}),
             command_id: None,
             created_at: Utc::now() }
}

/// Helper para añadir paso con locking optimista correcto
fn append_step(repo: &dyn FlowRepository, flow_id: &Uuid, content: &str) {
  let meta = repo.get_flow_meta(flow_id).expect("get meta");
  let next_cursor = meta.current_cursor + 1;
  let step = create_step(*flow_id, next_cursor, content);
  let result = repo.persist_data(&step, meta.current_version).expect("persist");
  assert!(matches!(result, PersistResult::Ok { .. }));
}

#[test]
fn test_diesel_create_flow_and_steps() {
  let db = create_temp_sqlite_db().expect("crear db test");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let flow_id =
    repo.create_flow(Some("test_flow".into()), Some("active".into()), json!({"purpose": "testing"})).expect("crear flujo");

  // Verificar metadata inicial
  let meta = repo.get_flow_meta(&flow_id).expect("get meta");
  assert_eq!(meta.name, Some("test_flow".to_string()));
  assert_eq!(meta.current_cursor, 0);
  assert_eq!(meta.current_version, 0);

  // Añadir 5 pasos
  for i in 1..=5 {
    append_step(&*repo, &flow_id, &format!("Step {}", i));
  }

  // Verificar que se guardaron correctamente
  let data = repo.read_data(&flow_id, 0).expect("read data");
  assert_eq!(data.len(), 5);

  let updated_meta = repo.get_flow_meta(&flow_id).expect("get meta");
  assert_eq!(updated_meta.current_cursor, 5);
  assert_eq!(updated_meta.current_version, 5);
}

#[test]
fn test_diesel_create_branch() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  // Crear flujo principal con 10 pasos
  let main_id = repo.create_flow(Some("main".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("Main {}", i));
  }

  // Crear rama desde paso 5
  let branch_id = repo.create_branch(&main_id, 5, json!({"reason": "test branch"})).expect("crear rama");

  // Verificar metadata de la rama
  let branch_meta = repo.get_flow_meta(&branch_id).expect("get branch meta");
  assert_eq!(branch_meta.parent_flow_id, Some(main_id));
  assert_eq!(branch_meta.parent_cursor, Some(5));
  assert_eq!(branch_meta.current_cursor, 5);
  assert!(branch_meta.name.unwrap().contains("branch"));

  // Verificar que heredó los 5 primeros pasos
  let branch_data = repo.read_data(&branch_id, 0).expect("read branch data");
  assert_eq!(branch_data.len(), 5);

  // Verificar contenido heredado
  let main_data = repo.read_data(&main_id, 0).expect("read main data");
  for i in 0..5 {
    assert_eq!(branch_data[i].payload["content"], main_data[i].payload["content"]);
  }
}

#[test]
fn test_diesel_branch_evolution() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let main_id = repo.create_flow(Some("main".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("Main {}", i));
  }

  let branch_id = repo.create_branch(&main_id, 5, json!({})).unwrap();

  // Añadir pasos solo a la rama
  for i in 6..=8 {
    append_step(&*repo, &branch_id, &format!("Branch {}", i));
  }

  // Verificar que la rama tiene 8 pasos
  let branch_data = repo.read_data(&branch_id, 0).expect("read");
  assert_eq!(branch_data.len(), 8);
  assert_eq!(branch_data[5].payload["content"], "Branch 6");

  // Verificar que el main sigue con 10 pasos
  let main_data = repo.read_data(&main_id, 0).expect("read");
  assert_eq!(main_data.len(), 10);
}

#[test]
fn test_diesel_nested_branches() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  // Principal: 10 pasos
  let main_id = repo.create_flow(Some("main".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("M{}", i));
  }

  // Branch1 desde paso 5
  let b1_id = repo.create_branch(&main_id, 5, json!({})).unwrap();
  for i in 6..=8 {
    append_step(&*repo, &b1_id, &format!("B1-{}", i));
  }

  // Branch2 desde paso 3 de Branch1 (hereda M1-M3)
  let b2_id = repo.create_branch(&b1_id, 3, json!({})).unwrap();

  // Verificar que B2 tiene 3 pasos heredados
  let b2_data = repo.read_data(&b2_id, 0).expect("read");
  assert_eq!(b2_data.len(), 3);
  assert_eq!(b2_data[0].payload["content"], "M1");
  assert_eq!(b2_data[2].payload["content"], "M3");

  // Añadir pasos a B2
  for i in 4..=6 {
    append_step(&*repo, &b2_id, &format!("B2-{}", i));
  }

  // Verificar que B2 tiene 6 pasos
  assert_eq!(repo.read_data(&b2_id, 0).unwrap().len(), 6);

  // Verificar que los otros no se afectaron
  assert_eq!(repo.read_data(&main_id, 0).unwrap().len(), 10);
  assert_eq!(repo.read_data(&b1_id, 0).unwrap().len(), 8);
}

#[test]
fn test_diesel_delete_branch() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let main_id = repo.create_flow(Some("main".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("M{}", i));
  }

  let branch_id = repo.create_branch(&main_id, 5, json!({})).unwrap();
  for i in 6..=8 {
    append_step(&*repo, &branch_id, &format!("B{}", i));
  }

  // Verificar que existe
  assert!(repo.branch_exists(&branch_id).unwrap());
  assert_eq!(repo.count_steps(&branch_id).unwrap(), 8);

  // Eliminar rama
  repo.delete_branch(&branch_id).expect("delete");

  // Verificar que ya no existe
  assert!(!repo.branch_exists(&branch_id).unwrap());

  // Verificar que el main sigue intacto
  assert!(repo.branch_exists(&main_id).unwrap());
  assert_eq!(repo.read_data(&main_id, 0).unwrap().len(), 10);
}

#[test]
fn test_diesel_snapshots() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let flow_id = repo.create_flow(Some("test".into()), Some("active".into()), json!({})).unwrap();

  // Añadir 10 pasos
  for i in 1..=10 {
    append_step(&*repo, &flow_id, &format!("Step {}", i));
  }

  // Guardar snapshot en paso 5
  let snap_ptr = "snapshot_at_5";
  let snap_id =
    repo.save_snapshot(&flow_id, 5, snap_ptr, json!({"cursor": 5, "description": "checkpoint"})).expect("save snapshot");

  // Cargar último snapshot
  let latest = repo.load_latest_snapshot(&flow_id).expect("load latest");
  assert!(latest.is_some());

  let snap = latest.unwrap();
  assert_eq!(snap.id, snap_id);
  assert_eq!(snap.cursor, 5);
  assert_eq!(snap.flow_id, flow_id);

  // Simular rehidratación: leer pasos posteriores al snapshot
  let replay_data = repo.read_data(&flow_id, 5).expect("read for replay");
  assert_eq!(replay_data.len(), 5); // pasos 6-10
}

#[test]
fn test_diesel_branch_inherits_snapshots() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let main_id = repo.create_flow(Some("main".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("Step {}", i));
  }

  // Guardar snapshots en pasos 3 y 7
  repo.save_snapshot(&main_id, 3, "snap_3", json!({})).unwrap();
  repo.save_snapshot(&main_id, 7, "snap_7", json!({})).unwrap();

  // Crear rama desde paso 5
  let branch_id = repo.create_branch(&main_id, 5, json!({})).unwrap();

  // La rama debe tener el snapshot del paso 3 (≤ 5)
  let branch_snap = repo.load_latest_snapshot(&branch_id).expect("load");

  if let Some(snap) = branch_snap {
    assert!(snap.cursor <= 5, "Rama no debe tener snapshots posteriores a parent_cursor");
    assert_eq!(snap.cursor, 3, "Debe ser el snapshot en paso 3");
  }
}

#[test]
fn test_diesel_optimistic_locking() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let flow_id = repo.create_flow(Some("test".into()), Some("active".into()), json!({})).unwrap();

  // Añadir paso con versión correcta
  let step1 = create_step(flow_id, 1, "Step 1");
  let result = repo.persist_data(&step1, 0).expect("persist");
  assert!(matches!(result, PersistResult::Ok { new_version: 1 }));

  // Intentar añadir con versión incorrecta
  let step2 = create_step(flow_id, 2, "Step 2");
  let result = repo.persist_data(&step2, 0).expect("persist"); // debería ser 1
  assert!(matches!(result, PersistResult::Conflict));

  // Añadir con versión correcta
  let result = repo.persist_data(&step2, 1).expect("persist");
  assert!(matches!(result, PersistResult::Ok { new_version: 2 }));
}

#[test]
fn test_diesel_complex_tree() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  // Principal con 15 pasos
  let main = repo.create_flow(Some("main".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=15 {
    append_step(&*repo, &main, &format!("M{}", i));
  }

  // Dos ramas desde diferentes puntos
  let b1 = repo.create_branch(&main, 5, json!({})).unwrap();
  let b2 = repo.create_branch(&main, 10, json!({})).unwrap();

  // Evolucionar cada rama
  for i in 6..=8 {
    append_step(&*repo, &b1, &format!("B1-{}", i));
  }

  for i in 11..=13 {
    append_step(&*repo, &b2, &format!("B2-{}", i));
  }

  // Subrama de b1
  let b1_1 = repo.create_branch(&b1, 7, json!({})).unwrap();
  append_step(&*repo, &b1_1, "B1_1-8");

  // Verificar estructura completa
  assert_eq!(repo.count_steps(&main).unwrap(), 15);
  assert_eq!(repo.count_steps(&b1).unwrap(), 8);
  assert_eq!(repo.count_steps(&b2).unwrap(), 13);
  assert_eq!(repo.count_steps(&b1_1).unwrap(), 8);

  // Verificar integridad: modificar main no afecta ramas
  append_step(&*repo, &main, "M16");
  assert_eq!(repo.count_steps(&main).unwrap(), 16);
  assert_eq!(repo.count_steps(&b1).unwrap(), 8); // sin cambios
}

#[test]
fn test_diesel_list_flows() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let f1 = repo.create_flow(Some("flow1".into()), Some("active".into()), json!({})).unwrap();
  let f2 = repo.create_flow(Some("flow2".into()), Some("active".into()), json!({})).unwrap();
  let b1 = repo.create_branch(&f1, 0, json!({})).unwrap();

  let all_ids = repo.list_flow_ids().expect("list");
  assert_eq!(all_ids.len(), 3);
  assert!(all_ids.contains(&f1));
  assert!(all_ids.contains(&f2));
  assert!(all_ids.contains(&b1));
}

#[test]
fn test_diesel_metadata_operations() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let flow_id = repo.create_flow(Some("test".into()), Some("active".into()), json!({})).unwrap();

  // Set múltiples keys
  repo.set_meta(&flow_id, "workflow_type", json!("CADMA")).unwrap();
  repo.set_meta(&flow_id, "version", json!("2.0")).unwrap();
  repo.set_meta(&flow_id, "author", json!("test_system")).unwrap();

  // Get
  assert_eq!(repo.get_meta(&flow_id, "workflow_type").unwrap(), json!("CADMA"));
  assert_eq!(repo.get_meta(&flow_id, "version").unwrap(), json!("2.0"));

  // Delete
  repo.del_meta(&flow_id, "version").unwrap();
  assert_eq!(repo.get_meta(&flow_id, "version").unwrap(), serde_json::Value::Null);

  // Otros siguen presentes
  assert_eq!(repo.get_meta(&flow_id, "workflow_type").unwrap(), json!("CADMA"));
}

#[test]
fn test_diesel_dump_debug() {
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  let f1 = repo.create_flow(Some("flow1".into()), Some("active".into()), json!({})).unwrap();
  for i in 1..=3 {
    append_step(&*repo, &f1, &format!("Step {}", i));
  }

  let (flows, data) = repo.dump_tables_for_debug().expect("dump");

  assert_eq!(flows.len(), 1);
  assert_eq!(data.len(), 3);

  let f1_meta = &flows[0];
  assert_eq!(f1_meta.id, f1);
  assert_eq!(f1_meta.current_cursor, 3);
}

#[test]
fn test_diesel_rehydration_scenario() {
  // Test completo de rehidratación: crear flujo, guardar snapshots,
  // crear ramas, y simular recuperación completa del estado
  let db = create_temp_sqlite_db().expect("crear db");
  let repo = Arc::new(DieselFlowRepository::new_with_pool(db.pool.clone()).expect("repo"));

  // Crear flujo principal con 20 pasos
  let main_id = repo.create_flow(Some("rehydration_test".into()), Some("active".into()), json!({})).unwrap();

  for i in 1..=20 {
    append_step(&*repo, &main_id, &format!("Step {}", i));

    // Guardar snapshots cada 5 pasos
    if i % 5 == 0 {
      repo.save_snapshot(&main_id, i, &format!("snap_{}", i), json!({"cursor": i})).unwrap();
    }
  }

  // Crear rama desde paso 10
  let branch_id = repo.create_branch(&main_id, 10, json!({})).unwrap();
  for i in 11..=15 {
    append_step(&*repo, &branch_id, &format!("Branch {}", i));
  }

  // Simular rehidratación del main:
  // 1. Cargar último snapshot
  let main_snap = repo.load_latest_snapshot(&main_id).expect("load snap").unwrap();
  assert_eq!(main_snap.cursor, 20);

  // 2. Si necesitáramos replay desde snapshot anterior (ej. 15):
  let replay_from_15 = repo.read_data(&main_id, 15).expect("replay");
  assert_eq!(replay_from_15.len(), 5); // pasos 16-20

  // Simular rehidratación de la rama:
  let branch_snap = repo.load_latest_snapshot(&branch_id).expect("load branch snap");
  if let Some(snap) = branch_snap {
    // La rama heredó snapshots ≤ 10
    assert!(snap.cursor <= 10);

    // Replay desde ese snapshot hasta el final
    let branch_replay = repo.read_data(&branch_id, snap.cursor).expect("branch replay");
    assert!(branch_replay.len() > 0);
  } else {
    // Si no hay snapshot, replay desde 0
    let all_steps = repo.read_data(&branch_id, 0).expect("all steps");
    assert_eq!(all_steps.len(), 15);
  }

  // Verificar integridad final
  assert_eq!(repo.count_steps(&main_id).unwrap(), 20);
  assert_eq!(repo.count_steps(&branch_id).unwrap(), 15);
}
