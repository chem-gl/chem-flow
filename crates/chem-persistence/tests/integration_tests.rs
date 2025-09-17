use chem_persistence::new_from_env;
use chem_persistence::DieselFlowRepository;
use flow::repository::FlowRepository;

use chrono::Utc;
use flow::domain::FlowData;
use serde_json::json;
use uuid::Uuid;

fn setup_repo() -> DieselFlowRepository {
    // Forzar uso de SQLite in-memory y usar new_from_env que ejecuta migraciones
    std::env::set_var("DATABASE_URL", "file:memdb1?mode=memory&cache=shared");
    let repo = new_from_env().expect("new_from_env");
    repo
}

#[test]
fn test_create_and_persist_flow_data_and_branching() {
    let repo = setup_repo();
    let flow_id = repo.create_flow(Some("mi-flow".into()), Some("running".into()), json!({"k":"v"}))
                      .expect("create");
    assert!(repo.branch_exists(&flow_id).unwrap());
    // persist some steps
    let now = Utc::now();
    for i in 1..=3 {
        let fd = FlowData { id: Uuid::new_v4(),
                            flow_id,
                            cursor: i,
                            key: "step-result".into(),
                            payload: json!({"i": i}),
                            metadata: json!({}),
                            command_id: None,
                            created_at: now };
        let res = repo.persist_data(&fd, (i - 1) as i64).expect("persist");
        match res {
            flow::domain::PersistResult::Ok { new_version } => assert!(new_version >= 1),
            flow::domain::PersistResult::Conflict => panic!("conflict"),
        }
    }
    // read back
    let items = repo.read_data(&flow_id, 0).expect("read");
    assert_eq!(items.len(), 3);
    // create branch at cursor 2
    let branch_id = repo.create_branch(&flow_id, Some("rama".into()), None, 2, json!({}))
                        .expect("branch");
    assert!(repo.branch_exists(&branch_id).unwrap());
    // Verificar que la rama guarda parent_flow_id correctamente
    let meta = repo.get_flow_meta(&branch_id).expect("get meta");
    assert_eq!(meta.parent_flow_id.unwrap(), flow_id);
    // branch should have 2 steps
    let count = repo.count_steps(&branch_id).expect("count");
    assert_eq!(count, 2);

    // --- crear y eliminar rama temporal ---
    let temp_branch = repo.create_branch(&flow_id, Some("temp-branch".into()), Some("queued".into()), 2, json!({}))
                          .expect("create temp");
    assert!(repo.branch_exists(&temp_branch).unwrap());
    // Añadir un paso para tener datos
    let now = Utc::now();
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: temp_branch,
                        cursor: 3,
                        key: "step".into(),
                        payload: json!({"m":1}),
                        metadata: json!({}),
                        command_id: None,
                        created_at: now };
    let _ = repo.persist_data(&fd, 0).expect("persist temp");
    assert!(repo.branch_exists(&temp_branch).unwrap());
    assert_eq!(repo.count_steps(&temp_branch).unwrap(), 3);
    // eliminar
    repo.delete_branch(&temp_branch).expect("delete branch");
    assert!(!repo.branch_exists(&temp_branch).unwrap());

    // Dump final tables para inspección manual
    let (flows, data) = repo.dump_tables_for_debug().expect("dump");
    println!("flows dump: {} rows", flows.len());
    for f in &flows {
        println!("flow: id={} parent={:?} cursor={} version={}",
                 f.id, f.parent_flow_id, f.current_cursor, f.current_version);
    }
    println!("flow_data dump: {} rows", data.len());
    for d in &data {
        println!("data: flow={} cursor={} key={} payload={}",
                 d.flow_id, d.cursor, d.key, d.payload);
    }
}
