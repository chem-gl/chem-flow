#![cfg(not(feature = "pg"))]
use chem_persistence::DieselFlowRepository;
use chrono::Utc;
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use serde_json::json;
use uuid::Uuid;
// Cuando el crate se compila con la feature `pg`, el harness de tests puede
// seguir siendo compilado como test de integración. Proporcionar dos variantes
// `setup_repo` para que se use el constructor correcto dependiendo de la
// feature habilitada.
#[cfg(not(feature = "pg"))]
fn setup_repo() -> DieselFlowRepository {
  // Usar una base en memoria con nombre único por test para aislarlos
  // y evitar bloqueos entre pruebas que puedan correr en paralelo.
  let url = format!("file:memdb_{}?mode=memory&cache=shared", Uuid::new_v4());
  std::env::set_var("DATABASE_URL", &url);
  DieselFlowRepository::new(&url)
}
#[cfg(feature = "pg")]
fn setup_repo() -> DieselFlowRepository {
  // Para builds con feature `pg`, intentar usar el constructor PG. El runner de
  // tests debe establecer una DATABASE_URL real apuntando a una instancia de
  // Postgres de test. Si no se establece, `new_pg` retornará un Err y el test
  // fallará rápido con un mensaje útil.
  dotenvy::dotenv().ok();
  let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for pg tests");
  DieselFlowRepository::new_pg(&url).expect("create pg repo")
}
#[test]
fn test_create_and_persist_flow_data_and_branching() {
  let repo = setup_repo();
  let flow_id = repo.create_flow(Some("mi-flow".into()), Some("running".into()), json!({"k":"v"})).expect("create");
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
  // crear rama en cursor 2
  let branch_id = repo.create_branch(&flow_id, Some("rama".into()), None, 2, json!({})).expect("branch");
  assert!(repo.branch_exists(&branch_id).unwrap());
  // Verificar que la rama guarda parent_flow_id correctamente
  let meta = repo.get_flow_meta(&branch_id).expect("get meta");
  assert_eq!(meta.parent_flow_id.unwrap(), flow_id);
  // branch should have 2 steps
  let count = repo.count_steps(&branch_id).expect("count");
  assert_eq!(count, 2);
  // --- crear y eliminar rama temporal ---
  let temp_branch =
    repo.create_branch(&flow_id, Some("temp-branch".into()), Some("queued".into()), 2, json!({})).expect("create temp");
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
#[test]
fn child_preserves_steps_after_parent_deletion_sqlite() {
  let repo = setup_repo();
  let parent = repo.create_flow(Some("parent-sql".into()), None, json!({"p":"v"})).expect("create");
  // add steps
  let mut expected = 0i64;
  for i in 1..=5 {
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: parent,
                        cursor: i,
                        key: "Step".into(),
                        payload: json!({"v": i}),
                        metadata: json!({"m": i}),
                        command_id: None,
                        created_at: Utc::now() };
    match repo.persist_data(&fd, expected).expect("persist") {
      flow::domain::PersistResult::Ok { new_version } => expected = new_version,
      _ => panic!("persist failed"),
    }
  }
  // create child clone
  let child = repo.create_branch(&parent, Some("child-sql".into()), None, 5, json!({})).expect("branch");
  #[cfg(not(feature = "pg"))]
  assert_eq!(repo.count_steps(&child).unwrap(), 5);
  // delete parent; child should remain with its cloned steps
  repo.delete_branch(&parent).expect("delete parent");
  assert!(!repo.branch_exists(&parent).unwrap());
  assert!(repo.branch_exists(&child).unwrap());
  assert_eq!(repo.count_steps(&child).unwrap(), 5);
  let items = repo.read_data(&child, 0).expect("read child");
  assert_eq!(items[0].metadata["m"].as_i64().unwrap(), 1);
}
