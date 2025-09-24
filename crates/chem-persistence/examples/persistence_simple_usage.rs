use chem_persistence::new_flow_from_env;
use chrono::Utc;
use flow::domain::FlowData;
use flow::repository::FlowRepository;
use serde_json::json;
use uuid::Uuid;

fn main() {
  // El ejemplo por defecto usa la variable de entorno DATABASE_URL.
  // Para demos locales rápidos puedes exportar:
  //   export DATABASE_URL="file:memdb1?mode=memory&cache=shared"
  // Si no se proporciona, `new_from_env` intentará conectarse a Postgres
  // usando la URL definida en el entorno (útil para demostraciones con PG).
  let repo = new_flow_from_env().expect("no se pudo inicializar el repositorio");

  // Crear flow
  let flow_id = repo.create_flow(Some("example".into()), Some("queued".into()), json!({})).expect("create");
  println!("created flow {}", flow_id);

  // Añadir 6 pasos al flujo original
  let now = Utc::now();
  for i in 1..=6 {
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id,
                        cursor: i,
                        key: "step-result".into(),
                        payload: json!({"i": i}),
                        metadata: json!({}),
                        command_id: None,
                        created_at: now };
    let _ = repo.persist_data(&fd, i - 1).expect("persist");
  }
  let items = repo.read_data(&flow_id, 0).expect("read");
  println!("original items: {}", items.len());

  // Crear rama desde cursor 6
  let parent_cursor = 6;
  let new_flow_id =
    repo.create_branch(&flow_id, Some("rama1".into()), Some("queued".into()), parent_cursor, json!({})).expect("branch");
  println!("created branch {} from {}@{}", new_flow_id, flow_id, parent_cursor);

  // Añadir 3 pasos en la rama
  for j in 1..=3 {
    let cursor = parent_cursor + j;
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: new_flow_id,
                        cursor,
                        key: "step-result".into(),
                        payload: json!({"j": j}),
                        metadata: json!({}),
                        command_id: None,
                        created_at: now };
    let _ = repo.persist_data(&fd, j - 1).expect("persist branch");
  }

  let branch_items = repo.read_data(&new_flow_id, 0).expect("read branch");
  println!("branch items: {}", branch_items.len());

  // Crear otra rama desde paso 3 del original
  let branch3 = repo.create_branch(&flow_id, Some("branch3".into()), Some("queued".into()), 3, json!({})).expect("branch3");
  println!("created branch3 {} from {}@3", branch3, flow_id);
  for m in 1..=2 {
    let cursor = 3 + m;
    let fd = FlowData { id: Uuid::new_v4(),
                        flow_id: branch3,
                        cursor,
                        key: "step-result".into(),
                        payload: json!({"m": m}),
                        metadata: json!({}),
                        command_id: None,
                        created_at: now };
    let _ = repo.persist_data(&fd, m - 1).expect("persist b3");
  }

  // Crear y eliminar una rama temporal
  let temp_branch =
    repo.create_branch(&flow_id, Some("temp-branch".into()), Some("queued".into()), 2, json!({})).expect("temp");
  println!("created temp {}", temp_branch);
  let fd = FlowData { id: Uuid::new_v4(),
                      flow_id: temp_branch,
                      cursor: 3,
                      key: "step".into(),
                      payload: json!({"m":1}),
                      metadata: json!({}),
                      command_id: None,
                      created_at: now };
  let _ = repo.persist_data(&fd, 0).expect("persist temp");
  println!("temp branch steps before delete: {}", repo.count_steps(&temp_branch).unwrap());
  repo.delete_branch(&temp_branch).expect("delete temp");
  println!("temp branch exists after delete: {}", repo.branch_exists(&temp_branch).unwrap());

  // Dump tables
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
