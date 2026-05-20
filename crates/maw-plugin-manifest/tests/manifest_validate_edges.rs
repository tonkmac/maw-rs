use maw_plugin_manifest::{parse_api, parse_cli, ApiMethod, CliFlagKind};
use serde_json::json;

#[test]
fn parse_cli_rejects_malformed_cli_shapes_and_preserves_optional_fields() {
    assert_eq!(parse_cli(&json!({})).expect("missing cli is valid"), None);

    let parsed = parse_cli(&json!({
        "cli": {
            "command": "demo",
            "aliases": ["d"],
            "help": "hi",
            "flags": { "verbose": "boolean" }
        }
    }))
    .expect("valid cli")
    .expect("cli present");
    assert_eq!(parsed.command, "demo");
    assert_eq!(parsed.aliases, Some(vec!["d".to_owned()]));
    assert_eq!(parsed.help, Some("hi".to_owned()));
    assert_eq!(
        parsed.flags.expect("flags present").get("verbose"),
        Some(&CliFlagKind::Boolean)
    );

    expect_error(&json!({ "cli": [] }), "plugin.json: cli must be an object");
    expect_error(
        &json!({ "cli": { "command": "" } }),
        "plugin.json: cli.command must be a non-empty string",
    );
    expect_error(
        &json!({ "cli": { "command": "x", "aliases": [1] } }),
        "plugin.json: cli.aliases must be an array of strings",
    );
    expect_error(
        &json!({ "cli": { "command": "x", "flags": [] } }),
        "plugin.json: cli.flags must be an object",
    );
    expect_error(
        &json!({ "cli": { "command": "x", "flags": { "bad": "object" } } }),
        "plugin.json: cli.flags[\"bad\"] must be \"boolean\", \"string\", or \"number\"",
    );
}

#[test]
fn parse_api_rejects_malformed_api_objects() {
    assert_eq!(parse_api(&json!({})).expect("missing api is valid"), None);

    let parsed = parse_api(&json!({
        "api": { "path": "/api/demo", "methods": ["GET", "POST"] }
    }))
    .expect("valid api")
    .expect("api present");
    assert_eq!(parsed.path, "/api/demo");
    assert_eq!(parsed.methods, vec![ApiMethod::Get, ApiMethod::Post]);

    assert_eq!(ApiMethod::Get.as_str(), "GET");
    assert_eq!(CliFlagKind::Number.as_str(), "number");

    expect_api_error(&json!({ "api": [] }), "plugin.json: api must be an object");
    expect_api_error(
        &json!({ "api": { "path": "", "methods": ["GET"] } }),
        "plugin.json: api.path must be a non-empty string",
    );
    expect_api_error(
        &json!({ "api": { "path": "/api/demo", "methods": ["PUT"] } }),
        "plugin.json: api.methods must be an array",
    );
}

fn expect_error(input: &serde_json::Value, expected: &str) {
    let error = parse_cli(input).expect_err("expected parse_cli error");
    assert!(
        error.contains(expected),
        "{error:?} did not contain {expected:?}"
    );
}

fn expect_api_error(input: &serde_json::Value, expected: &str) {
    let error = parse_api(input).expect_err("expected parse_api error");
    assert!(
        error.contains(expected),
        "{error:?} did not contain {expected:?}"
    );
}
