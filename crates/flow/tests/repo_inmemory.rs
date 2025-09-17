use chrono::Utc;
use flow::domain::FlowData;
use flow::domain::PersistResult;
use flow::stubs::InMemoryFlowRepository;
use flow::FlowRepository;
use serde_json::json;
use uuid::Uuid;

#[test]
fn delete_branch_removes_subtree() {
    let repo = InMemoryFlowRepository::new();
    let parent = repo.create_flow(Some("parent".into()), Some("queued".into()), json!({}))
                     .unwrap();

    // append 3 steps
    let mut expected_version = 0i64;
    for i in 1..=3 {
        let d = FlowData { id: Uuid::new_v4(),
                           flow_id: parent,
                           cursor: i,
                           key: "Step".into(),
                           payload: json!({"step": i}),
                           metadata: json!({}),
                           command_id: None,
                           created_at: Utc::now() };
        match repo.persist_data(&d, expected_version).unwrap() {
            PersistResult::Ok { new_version } => expected_version = new_version,
            PersistResult::Conflict => panic!("unexpected conflict"),
        }
    }

    // create child branch from cursor 3
    let child = repo.create_branch(&parent, Some("child".into()), None, 3, json!({})).unwrap();
    assert!(repo.branch_exists(&child).unwrap());

    // delete child and ensure it's gone but parent remains
    repo.delete_branch(&child).unwrap();
    assert!(!repo.branch_exists(&child).unwrap());
    assert!(repo.branch_exists(&parent).unwrap());
}

#[test]
fn delete_from_step_prunes_and_removes_subbranches() {
    let repo = InMemoryFlowRepository::new();
    let parent = repo.create_flow(Some("parent2".into()), None, json!({})).unwrap();

    // append 6 steps
    let mut expected = 0i64;
    for i in 1..=6 {
        let d = FlowData { id: Uuid::new_v4(),
                           flow_id: parent,
                           cursor: i,
                           key: "Step".into(),
                           payload: json!({"v": i}),
                           metadata: json!({}),
                           command_id: None,
                           created_at: Utc::now() };
        match repo.persist_data(&d, expected).unwrap() {
            PersistResult::Ok { new_version } => expected = new_version,
            PersistResult::Conflict => panic!("conflict"),
        }
    }

    let child = repo.create_branch(&parent, Some("prune-child".into()), None, 6, json!({}))
                    .unwrap();
    // verify both exist
    assert!(repo.branch_exists(&parent).unwrap());
    assert!(repo.branch_exists(&child).unwrap());

    // prune from cursor 3
    repo.delete_from_step(&parent, 3).unwrap();

    // parent should have only steps < 3 => 2 steps
    assert_eq!(repo.count_steps(&parent).unwrap(), 2);
    // child should be gone
    assert!(!repo.branch_exists(&child).unwrap());
}

#[test]
fn count_steps_nonexistent_returns_minus_one() {
    let repo = InMemoryFlowRepository::new();
    let random = Uuid::new_v4();
    assert_eq!(repo.count_steps(&random).unwrap(), -1);
}
