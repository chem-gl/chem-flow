use flow::engine::FlowEngineConfig;
use flow::service::FlowService;
use flow::stubs::InMemoryFlowRepository;
use flow::FlowEngine;
use serde_json::json;
use std::sync::Arc;

fn main() {
    // Config y repo
    let repo = Arc::new(InMemoryFlowRepository::new());
    let engine_config = FlowEngineConfig {};
    let service = FlowService::new(repo.clone(), engine_config);
    // Crear flow
    let flow_id = service.start_flow(Some("example".into()), Some("queued".into()), json!({}))
                         .expect("create flow");
    println!("created flow {}\n", flow_id);
    // Append data usando engine helper directamente
    // crear otra instancia de config para el engine (es pequeña y Copy no impl)
    let engine_config2 = FlowEngineConfig {};
    let engine = FlowEngine::new(repo.clone(), engine_config2);
    // Crear 6 pasos en el flujo original
    for i in 1..=6 {
        let payload = json!({"step": i, "message": format!("original-{}", i)});
        let metadata = json!({"source": "original"});
        let res = engine.append(flow_id, "Step", payload, metadata, None, (i - 1) as i64)
                        .expect("append");
        println!("append {} result: {:?}", i, res);
    }

    // Leer y mostrar los 6 pasos
    let items = service.read_data(flow_id, 0).expect("read");
    println!("original items: {:?}\n", items);

    // Crear una rama a partir del cursor 6
    let parent_cursor = 6;
    // crear branch desde snapshot/cursor: se pasa nombre, estado y cursor
    let new_flow_id = service.create_branch(flow_id,
                                            Some("testing branch".into()),
                                            Some("queued".into()),
                                            parent_cursor,
                                            json!({}))
                             .expect("create branch");
    println!("created branch {} from {}@{}", new_flow_id, flow_id, parent_cursor);

    // Añadir 3 pasos en la rama
    for j in 1..=3 {
        let payload = json!({"step": j, "message": format!("branch-{}", j)});
        let metadata = json!({"source": "branch"});
        // We must calculate expected_version for branch: initially 0
        let res = engine.append(new_flow_id, "Step", payload, metadata, None, (j - 1) as i64)
                        .expect("append-branch");
        println!("branch append {} result: {:?}", j, res);
    }

    // Leer datos de la rama
    let branch_items = service.read_data(new_flow_id, 0).expect("read-branch");
    println!("branch items: {:?}\n", branch_items);
    // Crear un branch a partir de la nueva rama en cursor 7 (grandchild)
    let grandparent_cursor = 7;
    let created_grand = service.create_branch(new_flow_id,
                                              Some("grandchild".into()),
                                              Some("queued".into()),
                                              grandparent_cursor,
                                              json!({}))
                               .expect("create grandchild");
    println!("created grandchild {} from {}@{}",
             created_grand, new_flow_id, grandparent_cursor);

    // Añadir 2 pasos en la grandchild
    for k in 1..=2 {
        let payload = json!({"step": k, "message": format!("grandchild-{}", k)});
        let metadata = json!({"source": "grandchild"});
        let res = engine.append(created_grand, "Step", payload, metadata, None, (k - 1) as i64)
                        .expect("append-grandchild");
        println!("grandchild append {} result: {:?}", k, res);
    }

    let grand_items = service.read_data(created_grand, 0).expect("read-grand");
    println!("grandchild items: {:?}\n", grand_items);

    // Crear otra rama del flujo principal a partir del paso 3
    let created_b3 = service.create_branch(flow_id, Some("branch3".into()), Some("queued".into()), 3, json!({}))
                            .expect("create branch3");
    println!("created branch3 {} from {}@3", created_b3, flow_id);

    // Añadir 2 pasos en branch3
    for m in 1..=2 {
        let payload = json!({"step": m, "message": format!("branch3-{}", m)});
        let metadata = json!({"source": "branch3"});
        let res = engine.append(created_b3, "Step", payload, metadata, None, (m - 1) as i64)
                        .expect("append-branch3");
        println!("branch3 append {} result: {:?}", m, res);
    }

    let b3_items = service.read_data(created_b3, 0).expect("read-branch3");
    println!("branch3 items: {:?}\n", b3_items);

    // Leer datos finales del flujo original
    let items = service.read_data(flow_id, 0).expect("read");
    println!("items: {:?}", items);
}
