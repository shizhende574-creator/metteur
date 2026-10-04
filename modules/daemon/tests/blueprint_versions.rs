use metteur_daemon::storage::{
    blueprint_files,
    persistence::{Db, cf},
    versioning::VersionManager,
};
use metteur_shared::Blueprint;

fn setup() -> (std::path::PathBuf, Db, VersionManager, Blueprint) {
    let root =
        std::env::temp_dir().join(format!("metteur-blueprint-versions-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db = Db::open(&root.join(".metteur/db")).unwrap();
    let versions = VersionManager::new(db.clone(), root.clone());
    let graph = metteur_shared::dsl::compile(
        "blueprint \"versions\"\nentry start: Start\ne: End\nstart -> e\n",
    )
    .unwrap();
    (root, db, versions, graph)
}

#[test]
fn saves_use_existing_file_history_and_content_identity() {
    let (root, db, versions, mut graph) = setup();
    let original = serde_json::to_vec(&graph).unwrap();
    let first =
        blueprint_files::save(&db, &versions, &graph, "plan.blueprint", &original, None).unwrap();
    let count = versions.list_snapshots().unwrap().len();
    assert_eq!(
        blueprint_files::save(&db, &versions, &graph, "plan.blueprint", &original, None).unwrap(),
        first
    );
    assert_eq!(versions.list_snapshots().unwrap().len(), count);
    std::fs::write(root.join("unrelated.txt"), "other file changed").unwrap();
    versions.create_snapshot("unrelated").unwrap();
    versions.verify_blueprint(&first).unwrap();
    graph.nodes[0].data = serde_json::json!({"new":1});
    let updated = serde_json::to_vec(&graph).unwrap();
    let second =
        blueprint_files::save(&db, &versions, &graph, "plan.blueprint", &updated, Some(&first))
            .unwrap();
    assert_ne!(first.blob_hash, second.blob_hash);
    assert!(versions.verify_blueprint(&first).is_err());
    assert_eq!(
        versions
            .file_at_snapshot(first.snapshot_id, "plan.blueprint")
            .unwrap()
            .unwrap()
            .1
            .as_bytes(),
        original
    );
    assert_eq!(
        versions
            .file_at_snapshot(second.snapshot_id, "plan.blueprint")
            .unwrap()
            .unwrap()
            .1
            .as_bytes(),
        updated
    );
    assert_eq!(blueprint_files::binding(&db, graph.id).unwrap(), Some(second));
    assert!(versions.file_history("plan.blueprint").unwrap().len() >= 2);
    assert_eq!(
        serde_json::from_slice::<Blueprint>(
            &db.get(cf::BLUEPRINTS, graph.id.as_bytes()).unwrap().unwrap()
        )
        .unwrap(),
        graph
    );
}

#[test]
fn conflicts_exclusions_and_mismatched_content_do_not_overwrite() {
    let (root, db, versions, graph) = setup();
    let bytes = serde_json::to_vec(&graph).unwrap();
    for uri in [".metteur/plan.blueprint", "target/plan.blueprint", "../plan.blueprint"] {
        assert!(blueprint_files::save(&db, &versions, &graph, uri, &bytes, None).is_err());
    }
    let first =
        blueprint_files::save(&db, &versions, &graph, "plan.blueprint", &bytes, None).unwrap();
    std::fs::write(root.join("plan.blueprint"), "external edit").unwrap();
    assert!(
        blueprint_files::save(&db, &versions, &graph, "plan.blueprint", &bytes, Some(&first))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(root.join("plan.blueprint")).unwrap(), "external edit");
    let mut other = graph.clone();
    other.id = uuid::Uuid::new_v4();
    assert!(
        blueprint_files::save(&db, &versions, &other, "other.blueprint", &bytes, None).is_err()
    );
    assert!(blueprint_files::save(&db, &versions, &graph, "copy.blueprint", &bytes, None).is_err());
    assert!(
        blueprint_files::save(
            &db,
            &versions,
            &other,
            "plan.blueprint",
            &serde_json::to_vec(&other).unwrap(),
            None
        )
        .is_err()
    );
    assert!(!root.join("other.blueprint").exists());
}

#[test]
fn legacy_database_graph_gets_a_file_only_on_explicit_save() {
    let (root, db, versions, graph) = setup();
    let bytes = serde_json::to_vec(&graph).unwrap();
    db.put(cf::BLUEPRINTS, graph.id.as_bytes(), &bytes).unwrap();
    assert!(blueprint_files::binding(&db, graph.id).unwrap().is_none());
    assert!(!root.join("chosen.blueprint").exists());
    blueprint_files::save(&db, &versions, &graph, "chosen.blueprint", &bytes, None).unwrap();
    assert_eq!(std::fs::read(root.join("chosen.blueprint")).unwrap(), bytes);
}

#[test]
fn canvas_file_keeps_its_representation_and_decorations() {
    let (root, db, versions, graph) = setup();
    let mut nodes: Vec<_> = graph.nodes.iter().map(|n| {
        let pin = |p: &metteur_shared::Pin| serde_json::json!({"id":p.id,"key":p.key,"name":p.name,
            "kind":match p.pin_type { metteur_shared::PinType::ExecInput=>"exec-in",metteur_shared::PinType::ExecOutput=>"exec-out",metteur_shared::PinType::DataInput=>"data-in",metteur_shared::PinType::DataOutput=>"data-out" },
            "type":p.data_type.to_string(),"default":p.default,"optional":p.optional,"choices":p.choices,"description":p.description});
        serde_json::json!({"id":n.id,"type":n.kind,"nodeType":n.node_type,"title":"custom title","data":n.data,
            "position":{"x":n.position.0,"y":n.position.1},
            "inputs":n.pins.iter().filter(|p| matches!(p.pin_type,metteur_shared::PinType::ExecInput|metteur_shared::PinType::DataInput)).map(pin).collect::<Vec<_>>(),
            "outputs":n.pins.iter().filter(|p| matches!(p.pin_type,metteur_shared::PinType::ExecOutput|metteur_shared::PinType::DataOutput)).map(pin).collect::<Vec<_>>()})
    }).collect();
    nodes.push(serde_json::json!({"id":"reference","type":"FileReference","title":"README.md"}));
    let doc = serde_json::json!({"id":graph.id,"name":graph.name,"entryNodeId":graph.entry_node_id,"nodes":nodes,
        "edges":graph.edges.iter().map(|e|serde_json::json!({"id":e.id,"source":e.source_node,"sourceHandle":e.source_pin,"target":e.target_node,"targetHandle":e.target_pin})).collect::<Vec<_>>()});
    let bytes = serde_json::to_vec_pretty(&doc).unwrap();
    assert_eq!(blueprint_files::decode(&bytes).unwrap(), graph);
    blueprint_files::save(&db, &versions, &graph, "canvas.blueprint", &bytes, None).unwrap();
    assert_eq!(std::fs::read(root.join("canvas.blueprint")).unwrap(), bytes);
}
