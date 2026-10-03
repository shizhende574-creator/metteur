use metteur_cli::print::node_catalog;

#[test]
fn catalog_displays_contract_and_legacy_state() {
    use metteur_proto::proto::{NodeKindInfo, NodeKindList, Pin};
    let mut list = NodeKindList {
        kinds: vec!["FutureNode".into()],
        ..Default::default()
    };
    assert!(node_catalog(&list).contains("Pin signatures unavailable"));
    list.signature_version = 1;
    assert!(node_catalog(&list).contains("Pin signatures unavailable"));
    list.infos.push(NodeKindInfo {
        kind: "FutureNode".into(),
        node_type: "Function".into(),
        dynamic_pins: true,
        pins: vec![Pin {
            name: "Count".into(),
            pin_type: "DataInput".into(),
            data_type: "int".into(),
            default_json: "3".into(),
            optional: true,
            choices: vec!["3".into()],
            description: "Count description".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let text = node_catalog(&list);
    assert!(text.contains("FutureNode [Function] (dynamic pins)"));
    assert!(text.contains("DataInput Count: int optional default=3 choices=3"));
    assert!(text.contains("Count description"));
    assert!(!text.contains("unavailable"));
}
