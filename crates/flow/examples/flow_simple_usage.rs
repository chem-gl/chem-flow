use flow::engine::FlowEngineConfig;
use flow::errors::FlowError;
use flow::stubs::InMemoryFlowRepository;
use flow::FlowEngine;
use flow::FlowRepository;
use serde_json::json;
use std::sync::Arc;

fn main() -> Result<(), FlowError> {
    // Config y repo
    let repo = Arc::new(InMemoryFlowRepository::new());
    let engine_config = FlowEngineConfig {};
    let engine = FlowEngine::new(repo.clone(), engine_config);
    // Crear flow usando sólo el engine
    let flow_id = engine.start_flow(Some("example".into()), Some("queued".into()), json!({}))?;
    println!("created flow {}\n", flow_id);
    // Crear 6 pasos en el flujo original
    for i in 1..=6 {
        let payload = json!({"step": i, "message": format!("original-{}", i)});
        let metadata = json!({"source": "original"});
        let res = engine.append(flow_id, "Step", payload, metadata, None, (i - 1) as i64)?;
        println!("append {} result: {:?}", i, res);
    }
    // Leer y mostrar los 6 pasos
    let items = engine.get_items(&flow_id, 0)?;
    println!("original items: {:?}\n", items);

    // Crear una rama a partir del cursor 6
    let parent_cursor = 6;
    // crear branch desde snapshot/cursor: se pasa nombre, estado y cursor
    let new_flow_id = engine.new_branch(&flow_id,
                                        Some("testing branch".into()),
                                        Some("queued".into()),
                                        parent_cursor,
                                        json!({}))?;
    println!("created branch {} from {}@{}", new_flow_id, flow_id, parent_cursor);

    // Añadir 3 pasos en la rama
    for j in 1..=3 {
        let payload = json!({"step": j, "message": format!("branch-{}", j)});
        let metadata = json!({"source": "branch"});
        // We must calculate expected_version for branch: initially 0
        let res = engine.append(new_flow_id, "Step", payload, metadata, None, (j - 1) as i64)?;
        println!("branch append {} result: {:?}", j, res);
    }

    // Leer datos de la rama
    let branch_items = engine.get_items(&new_flow_id, 0)?;
    println!("branch items: {:?}\n", branch_items);

    // Crear un branch a partir de la nueva rama en cursor 7 (grandchild)
    let grandparent_cursor = 7;
    let created_grand = engine.new_branch(&new_flow_id,
                                          Some("grandchild".into()),
                                          Some("queued".into()),
                                          grandparent_cursor,
                                          json!({}))?;
    println!("created grandchild {} from {}@{}",
             created_grand, new_flow_id, grandparent_cursor);

    // Añadir 2 pasos en la grandchild
    for k in 1..=2 {
        let payload = json!({"step": k, "message": format!("grandchild-{}", k)});
        let metadata = json!({"source": "grandchild"});
        let res = engine.append(created_grand, "Step", payload, metadata, None, (k - 1) as i64)?;
        println!("grandchild append {} result: {:?}", k, res);
    }

    let grand_items = engine.get_items(&created_grand, 0)?;
    println!("grandchild items: {:?}\n", grand_items);

    // Crear otra rama del flujo principal a partir del paso 3
    let created_b3 = engine.new_branch(&flow_id, Some("branch3".into()), Some("queued".into()), 3, json!({}))?;
    println!("created branch3 {} from {}@3", created_b3, flow_id);

    // Añadir 2 pasos en branch3
    for m in 1..=2 {
        let payload = json!({"step": m, "message": format!("branch3-{}", m)});
        let metadata = json!({"source": "branch3"});
        let res = engine.append(created_b3, "Step", payload, metadata, None, (m - 1) as i64)?;
        println!("branch3 append {} result: {:?}", m, res);
    }

    let b3_items = engine.get_items(&created_b3, 0)?;
    println!("branch3 items: {:?}\n", b3_items);

    // Leer datos finales del flujo original
    let items = engine.get_items(&flow_id, 0)?;
    println!("items: {:?}", items);

    // Mostrar todos los ids de flujos usando la nueva función del repositorio
    let all_ids = repo.list_flow_ids();
    match all_ids {
        Ok(ids) => println!("all flow ids: {:?}", ids),
        Err(e) => println!("failed to list flow ids: {:?}", e),
    }

    // --- Ejemplo: crear y eliminar una rama completa ---
    let temp_branch = engine.new_branch(&flow_id, Some("temp-branch".into()), Some("queued".into()), 2, json!({}))?;
    println!("created temp branch {} from {}@2", temp_branch, flow_id);
    // Añadir un paso para que tenga contenido
    engine.append(temp_branch, "Step", json!({"m":1}), json!({"source":"temp"}), None, 0)?;
    println!("temp branch exists before delete: {}", engine.branch_exists(&temp_branch)?);
    println!("temp branch steps before delete: {}", engine.count_steps(&temp_branch)?);
    // Eliminar la rama completa
    engine.delete_branch(&temp_branch)?;
    println!("temp branch exists after delete: {}", engine.branch_exists(&temp_branch)?);

    // --- Ejemplo: crear subramas y eliminar desde un paso específico ---
    // Crear una rama desde cursor 4
    let parent_for_prune = engine.new_branch(&flow_id, Some("prune-parent".into()), Some("queued".into()), 4, json!({}))?;
    println!("created prune-parent {} from {}@4", parent_for_prune, flow_id);
    // Añadir pasos 1..4 en la rama
    for i in 1..=4 {
        engine.append(parent_for_prune,
                      "Step",
                      json!({"p": i}),
                      json!({"source":"prune"}),
                      None,
                      (i - 1) as i64)?;
    }
    // Crear una subrama desde cursor 6 del padre
    let child_of_parent = engine.new_branch(&parent_for_prune,
                                            Some("child-of-prune".into()),
                                            Some("queued".into()),
                                            6,
                                            json!({}))?;
    println!("created child {} from {}@6", child_of_parent, parent_for_prune);
    engine.append(child_of_parent, "Step", json!({"c": 1}), json!({"source":"child"}), None, 0)?;
    println!("counts before prune: parent={} child={}",
             engine.count_steps(&parent_for_prune)?,
             engine.count_steps(&child_of_parent)?);

    // Ahora eliminar desde el paso 3 en el padre: esto debe borrar pasos >=3 y las
    // subramas creadas cuyo parent_cursor >=3
    engine.delete_from_step(&parent_for_prune, 3)?;
    println!("counts after prune: parent={} child_exists={}",
             engine.count_steps(&parent_for_prune)?,
             engine.branch_exists(&child_of_parent)?);

    Ok(())
}
