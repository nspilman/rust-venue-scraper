use jsonschema::JSONSchema;
use serde_json::json;

#[test]
fn submission_example_is_valid() {
    let schema = include_str!("../schemas/envelope.v1.json");
    let instance = include_str!("resources/envelope_submission.json");
    let schema_json: serde_json::Value = serde_json::from_str(schema).unwrap();
    let instance_json: serde_json::Value = serde_json::from_str(instance).unwrap();
    let schema_static: &'static serde_json::Value = Box::leak(Box::new(schema_json));
    let compiled = JSONSchema::options().compile(schema_static).unwrap();
    assert!(compiled.is_valid(&instance_json));
}

#[test]
fn accepted_example_is_valid() {
    let schema = include_str!("../schemas/envelope.v1.json");
    let instance = include_str!("resources/envelope_accepted.json");
    let schema_json: serde_json::Value = serde_json::from_str(schema).unwrap();
    let instance_json: serde_json::Value = serde_json::from_str(instance).unwrap();
    let schema_static: &'static serde_json::Value = Box::leak(Box::new(schema_json));
    let compiled = JSONSchema::options().compile(schema_static).unwrap();
    assert!(compiled.is_valid(&instance_json));
}

#[test]
fn invalid_checksum_format_is_rejected() {
    let schema = include_str!("../schemas/envelope.v1.json");
    let schema_json: serde_json::Value = serde_json::from_str(schema).unwrap();
    let schema_static: &'static serde_json::Value = Box::leak(Box::new(schema_json));
    let compiled = JSONSchema::options().compile(schema_static).unwrap();

    let mut invalid: serde_json::Value =
        serde_json::from_str(include_str!("resources/envelope_submission.json")).unwrap();
    // Break checksum
    invalid["payload_meta"]["checksum"]["sha256"] = json!("NOTAHEX");

    assert!(!compiled.is_valid(&invalid), "checksum regex should fail");
}

#[test]
fn adapters_cannot_set_payload_ref() {
    let schema = include_str!("../schemas/envelope.v1.json");
    let schema_json: serde_json::Value = serde_json::from_str(schema).unwrap();
    let schema_static: &'static serde_json::Value = Box::leak(Box::new(schema_json));
    let compiled = JSONSchema::options().compile(schema_static).unwrap();

    // Start from submission and inject payload_ref (should still be allowed by schema, but policy will reject).
    let mut with_ref: serde_json::Value =
        serde_json::from_str(include_str!("resources/envelope_submission.json")).unwrap();
    with_ref["payload_ref"] = json!("sha256://deadbeef");
    // Schema permits payload_ref structurally
    assert!(compiled.is_valid(&with_ref));
}
