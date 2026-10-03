//! Contracts are exercised through the compiler and the real executors, rather
//! than comparing two copies of a template table.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use metteur_daemon::execution::context::ExecutionContext;
use metteur_daemon::execution::{Interpreter, SharedBlueprint};
use metteur_daemon::llm::LlmClientFactory;
use metteur_daemon::registry::Registry;
use metteur_shared::dsl::compile_with_catalog;
use metteur_shared::model::validate::validate_with_catalog;
use metteur_shared::{DataType, PinType, Value};

#[test]
fn every_registered_executor_has_a_deterministic_contract() {
    let registry = Registry::with_builtins();
    let catalog = registry.node_signatures();
    assert_eq!(catalog.keys().cloned().collect::<Vec<_>>(), registry.node_kinds());
    for (kind, signature) in &catalog {
        assert_eq!(&signature.kind, kind);
        assert_eq!(&signature.executor_kind, kind);
        assert!(!signature.description.is_empty());
        assert!(!signature.pins.is_empty() || signature.dynamic_pins);
        let mut seen = HashSet::new();
        for pin in &signature.pins {
            assert!(seen.insert((format!("{:?}", pin.pin_type), &pin.name)), "{kind}");
            if pin.pin_type == PinType::DataInput {
                assert!(!pin.key.is_empty(), "{kind}");
            }
        }
        let blueprint = compile_with_catalog(&format!("entry n: {kind}"), &catalog).unwrap();
        assert_eq!(blueprint.nodes[0].pins.len(), signature.pins.len());
    }
    assert_eq!(
        serde_json::to_string(&catalog).unwrap(),
        serde_json::to_string(&registry.node_signatures()).unwrap()
    );
    assert!(!catalog.contains_key("FileReference"));
}

#[test]
fn live_catalog_follows_tool_registration_and_schema() {
    let registry = Arc::new(Registry::with_builtins());
    let initial = registry.authoring_catalog();
    for tool in registry.tools() {
        let signature = &initial[tool.name()];
        let properties = tool.parameters()["properties"].as_object().cloned().unwrap_or_default();
        for name in properties.keys() {
            assert!(
                signature.pins.iter().any(|p| p.name == *name && p.pin_type == PinType::DataInput),
                "{}.{name}",
                tool.name()
            );
        }
    }
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let registry = registry.clone();
            std::thread::spawn(move || {
                for _ in 0..30 {
                    let _ = registry.authoring_catalog();
                }
            })
        })
        .collect();
    let tool = registry.unregister_tool("ReadFile").unwrap();
    let removed = registry.authoring_catalog();
    assert!(compile_with_catalog("entry n: ReadFile", &removed).is_err());
    assert!(compile_with_catalog("entry n: ReadFile", &initial).is_ok());
    registry.register_tool(tool);
    for reader in readers {
        reader.join().unwrap();
    }
    assert!(registry.authoring_catalog().contains_key("ReadFile"));
}

#[test]
fn function_pins_come_from_the_current_library_and_draft_ids_are_unique() {
    let registry = Registry::with_builtins();
    let catalog = registry.authoring_catalog();
    for function in registry.functions() {
        let bp = compile_with_catalog(
            &format!("entry n: CallFunction(function = {:?})", function.name),
            &catalog,
        )
        .unwrap();
        for input in &function.signature.inputs {
            let pin = bp.nodes[0]
                .pins
                .iter()
                .find(|p| p.name == input.name && p.pin_type == PinType::DataInput)
                .unwrap();
            assert_eq!(pin.data_type, input.data_type);
            assert_eq!(pin.default, input.default);
            assert_eq!(pin.optional, input.optional);
        }
    }
    let draft = serde_json::json!({"nodes": {"llm": {"kind": "CallLLM"}}});
    let bp = metteur_shared::dsl::compile_draft_value_with_catalog(&draft, &catalog).unwrap();
    let ids: HashSet<_> = bp.nodes[0].pins.iter().map(|p| p.id).collect();
    assert_eq!(ids.len(), bp.nodes[0].pins.len());
}

#[tokio::test]
async fn compiled_contract_pins_match_scalar_executor_reads_and_outputs() {
    let registry = Arc::new(Registry::with_builtins());
    let catalog = registry.node_signatures();
    let mut ctx =
        ExecutionContext::new(registry.clone(), LlmClientFactory::new(), std::env::temp_dir());
    for kind in [
        "Add",
        "Subtract",
        "Multiply",
        "Divide",
        "Modulo",
        "Power",
        "Min",
        "Max",
        "Abs",
        "Round",
        "Equal",
        "NotEqual",
        "Greater",
        "Less",
        "GreaterEqual",
        "LessEqual",
        "And",
        "Or",
        "Xor",
        "Not",
        "Concat",
        "Length",
        "Upper",
        "Lower",
        "Trim",
        "Contains",
        "ToString",
        "ToInt",
        "ToFloat",
        "ToBool",
        "ToJson",
    ] {
        let blueprint = compile_with_catalog(&format!("entry n: {kind}"), &catalog).unwrap();
        let node = &blueprint.nodes[0];
        let inputs = node
            .pins
            .iter()
            .filter(|p| p.pin_type == PinType::DataInput)
            .map(|p| {
                let value = match p.data_type {
                    DataType::Bool => Value::Bool(true),
                    DataType::String => Value::String("3".into()),
                    _ => Value::Int(3),
                };
                (p.id, value)
            })
            .collect();
        let outputs = registry
            .node_executor(kind)
            .unwrap()
            .execute(node, &inputs, &mut ctx)
            .await
            .unwrap_or_else(|e| panic!("{kind}: {e}"));
        for (id, value) in outputs {
            let pin = node.pins.iter().find(|p| p.id == id).unwrap();
            assert_eq!(pin.pin_type, PinType::DataOutput, "{kind}");
            let actual = metteur_shared::model::types::data_type_of(&value);
            assert!(
                metteur_shared::compatible(&actual, &pin.data_type),
                "{kind}: {actual} -> {}",
                pin.data_type
            );
        }
    }
}

#[tokio::test]
async fn inline_values_win_over_defaults_and_optional_nulls_in_real_run() {
    let registry = Arc::new(Registry::with_builtins());
    let catalog = registry.authoring_catalog();
    let mut bp = compile_with_catalog("entry n: Add(A = 4, B = 3)", &catalog).unwrap();
    let a = bp.nodes[0].pins.iter_mut().find(|p| p.name == "A").unwrap();
    a.default = Some(serde_json::json!(99));
    a.optional = true;
    let report = validate_with_catalog(&bp, &catalog);
    assert!(report.is_ok());
    assert!(report.warnings.is_empty());
    let bp: SharedBlueprint = Arc::new(parking_lot::RwLock::new(bp));
    let mut runner = Interpreter::new(registry, LlmClientFactory::new(), std::env::temp_dir());
    let events = runner.run(&bp, None).await.unwrap();
    let text = format!("{events:?}");
    assert!(!text.contains("Error"), "{text}");
    assert!(text.contains("7.0"), "{text}");
}

#[test]
fn validation_uses_canonical_types_but_missing_inputs_remain_warnings() {
    let registry = Registry::with_builtins();
    let catalog = registry.node_signatures();
    let bp = compile_with_catalog("entry n: Add", &catalog).unwrap();
    let report = validate_with_catalog(&bp, &catalog);
    assert!(report.is_ok());
    assert_eq!(report.warnings.len(), 2);
    let mut bp = compile_with_catalog(
        "entry a: Add(A = 2, B = 3)\nb: Delay(Ms <- a.Result)\na -> b",
        &catalog,
    )
    .unwrap();
    // A stale client calls Ms a float. The daemon must still reject narrowing.
    bp.nodes[1].pins.iter_mut().find(|p| p.name == "Ms").unwrap().data_type = DataType::Float;
    assert!(!validate_with_catalog(&bp, &catalog).is_ok());
    let bp = compile_with_catalog(
        "entry a: Add(A = 2, B = 3)\nb: Upper(In <- a.Result)\na -> b",
        &catalog,
    )
    .unwrap();
    assert!(validate_with_catalog(&bp, &catalog).is_ok());
}

#[tokio::test]
async fn call_llm_context_input_and_output_are_distinct() {
    let registry = Arc::new(Registry::with_builtins());
    let bp = compile_with_catalog(
        "entry n: CallLLM(provider = \"mock\", mock_text = \"answer\")",
        &registry.node_signatures(),
    )
    .unwrap();
    let node = &bp.nodes[0];
    let input =
        node.pins.iter().find(|p| p.name == "Context" && p.pin_type == PinType::DataInput).unwrap();
    let output = node
        .pins
        .iter()
        .find(|p| p.name == "Context" && p.pin_type == PinType::DataOutput)
        .unwrap();
    let context =
        metteur_shared::ContextManager::new_from_prompt(Vec::new(), "retain this message");
    let mut ctx =
        ExecutionContext::new(registry.clone(), LlmClientFactory::new(), std::env::temp_dir());
    let result = registry
        .node_executor("CallLLM")
        .unwrap()
        .execute(node, &HashMap::from([(input.id, Value::Context(context))]), &mut ctx)
        .await
        .unwrap();
    assert!(!result.contains_key(&input.id));
    let context = result[&output.id].as_context().unwrap();
    assert!(context.messages.iter().any(|m| m.text_content().contains("retain this message")));
}
