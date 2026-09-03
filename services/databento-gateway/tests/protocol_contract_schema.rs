#![cfg(feature = "json-schema")]

use std::fs;
use std::path::PathBuf;

use databento_gateway::protocol::{
    BarPageResponse, ClientCommand, DatasetResponse, ErrorResponse, HistoryRequest,
    ResolveRequest, ResolveResponse, SearchRequest, SearchResponse, ServerEvent,
};
use schemars::generate::SchemaSettings;
use serde_json::{json, Map, Value};

const ROOT_ORDER: [&str; 10] = [
    "ClientCommand",
    "ServerEvent",
    "HistoryRequest",
    "ResolveRequest",
    "SearchRequest",
    "BarPageResponse",
    "ResolveResponse",
    "SearchResponse",
    "DatasetResponse",
    "ErrorResponse",
];

fn generate_schema_document() -> Value {
    let mut generator = SchemaSettings::draft2020_12().into_generator();
    let mut roots = Map::new();
    macro_rules! root {
        ($name:literal, $ty:ty) => {
            roots.insert(
                $name.to_string(),
                serde_json::to_value(generator.subschema_for::<$ty>())
                    .expect("root schema serializes"),
            );
        };
    }
    root!("ClientCommand", ClientCommand);
    root!("ServerEvent", ServerEvent);
    root!("HistoryRequest", HistoryRequest);
    root!("ResolveRequest", ResolveRequest);
    root!("SearchRequest", SearchRequest);
    root!("BarPageResponse", BarPageResponse);
    root!("ResolveResponse", ResolveResponse);
    root!("SearchResponse", SearchResponse);
    root!("DatasetResponse", DatasetResponse);
    root!("ErrorResponse", ErrorResponse);

    let definitions: Map<String, Value> = generator
        .take_definitions(true)
        .into_iter()
        .collect();

    let ordered_roots: Map<String, Value> = ROOT_ORDER
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                roots.remove(*name).expect("declared root generated"),
            )
        })
        .collect();

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Databento Lightweight Charts protocol v1",
        "description": "Machine-generated from services/databento-gateway/src/protocol.rs. Regenerate with: UPDATE_PROTOCOL_SCHEMA=1 cargo test -p databento-gateway --features json-schema protocol_contract_schema. Structural contract only; semantic invariants (safe-integer times, volume/bar time equality, event ordering) are enforced by the Rust and Zod validators.",
        "roots": ordered_roots,
        "$defs": definitions,
    })
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts/protocol-v1.schema.json")
}

#[test]
fn protocol_contract_schema_is_fresh() {
    let generated = generate_schema_document();
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&generated).expect("schema document serializes")
    );
    let path = schema_path();

    if std::env::var_os("UPDATE_PROTOCOL_SCHEMA").is_some() {
        fs::write(&path, &rendered).expect("write contracts/protocol-v1.schema.json");
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing {}; regenerate with UPDATE_PROTOCOL_SCHEMA=1 cargo test -p databento-gateway --features json-schema protocol_contract_schema",
            path.display()
        )
    });
    assert_eq!(
        committed, rendered,
        "contracts/protocol-v1.schema.json is stale; regenerate with UPDATE_PROTOCOL_SCHEMA=1 cargo test -p databento-gateway --features json-schema protocol_contract_schema"
    );
}
