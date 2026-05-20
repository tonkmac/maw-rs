use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use maw_cli::run_cli;
use serde_json::json;

fn json_output(output: &maw_cli::CliOutput) -> serde_json::Value {
    assert_eq!(output.code, 0, "{}", output.stderr);
    serde_json::from_str(&output.stdout).unwrap_or_else(|error| {
        panic!("invalid json: {error}\n{}", output.stdout);
    })
}

fn make_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "maw-rs-cli-plugin-manifest-{label}-{}-{nonce}",
        std::process::id()
    ));
    create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn plugin_manifest_parse_plan_cli_matches_maw_js_manifest_fixture() {
    let dir = make_temp_dir("parse-happy");
    write(dir.join("plugin.wasm"), b"wasm").expect("wasm");

    let manifest = json!({
        "name": "full-plugin",
        "version": "2.3.4",
        "wasm": "plugin.wasm",
        "sdk": "~1.2.0",
        "weight": 25,
        "cli": { "command": "greet", "help": "Say hello" },
        "api": { "path": "/greet", "methods": ["GET", "POST"] },
        "description": "A greeting plugin",
        "author": "Nat"
    });
    let output = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "parse".to_owned(),
        "--dir".to_owned(),
        dir.to_string_lossy().into_owned(),
        "--json".to_owned(),
        manifest.to_string(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(output["command"], "plugin-manifest");
    assert_eq!(output["kind"], "parse");
    assert_eq!(output["manifest"]["name"], "full-plugin");
    assert_eq!(output["manifest"]["version"], "2.3.4");
    assert_eq!(output["manifest"]["wasm"], "plugin.wasm");
    assert_eq!(output["manifest"]["sdk"], "~1.2.0");
    assert_eq!(output["manifest"]["weight"], 25);
    assert_eq!(output["manifest"]["cli"]["command"], "greet");
    assert_eq!(output["manifest"]["cli"]["help"], "Say hello");
    assert_eq!(output["manifest"]["api"]["path"], "/greet");
    assert_eq!(output["manifest"]["api"]["methods"], json!(["GET", "POST"]));
    assert_eq!(output["manifest"]["description"], "A greeting plugin");
    assert_eq!(output["manifest"]["author"], "Nat");

    remove_dir_all(dir).expect("cleanup");
}

#[test]
fn plugin_manifest_load_plan_cli_matches_maw_js_loader_contract() {
    let missing = make_temp_dir("missing");
    let output = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "load".to_owned(),
        "--dir".to_owned(),
        missing.to_string_lossy().into_owned(),
        "--plan-json".to_owned(),
    ]));
    assert_eq!(output["command"], "plugin-manifest");
    assert_eq!(output["kind"], "load");
    assert_eq!(output["present"], false);
    assert!(output["plugin"].is_null());
    remove_dir_all(missing).expect("cleanup missing");

    let dir = make_temp_dir("load-wasm");
    write(dir.join("plugin.wasm"), b"wasm").expect("wasm");
    write_manifest(
        &dir,
        &json!({ "name": "test-pkg", "version": "1.0.0", "wasm": "plugin.wasm", "sdk": "*" }),
    );

    let output = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "load".to_owned(),
        "--dir".to_owned(),
        dir.to_string_lossy().into_owned(),
        "--plan-json".to_owned(),
    ]));
    assert_eq!(output["present"], true);
    assert_eq!(output["plugin"]["kind"], "wasm");
    assert_eq!(output["plugin"]["disabled"], false);
    assert_eq!(output["plugin"]["manifest"]["name"], "test-pkg");
    assert_eq!(output["plugin"]["manifest"]["wasm"], "plugin.wasm");
    assert_eq!(
        output["plugin"]["wasmPath"],
        dir.join("plugin.wasm").to_string_lossy().as_ref()
    );
    assert!(output["plugin"]["entryPath"].is_null());

    remove_dir_all(dir).expect("cleanup load");
}

#[test]
fn plugin_manifest_load_plan_reports_entry_and_artifact_precedence() {
    let dir = make_temp_dir("entry-precedence");
    write(
        dir.join("index.ts"),
        b"export default () => ({ ok: true });\n",
    )
    .expect("entry");
    write(dir.join("plugin.wasm"), b"wasm").expect("wasm");
    write_manifest(
        &dir,
        &json!({
            "name": "source-first",
            "version": "1.0.0",
            "entry": "index.ts",
            "wasm": "plugin.wasm",
            "sdk": "*",
            "target": "js",
            "artifact": { "path": "dist/index.js", "sha256": null }
        }),
    );

    let output = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "load".to_owned(),
        "--dir".to_owned(),
        dir.to_string_lossy().into_owned(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(output["plugin"]["kind"], "ts");
    assert_eq!(
        output["plugin"]["entryPath"],
        dir.join("index.ts").to_string_lossy().as_ref()
    );
    assert_eq!(output["plugin"]["manifest"]["target"], "js");
    assert_eq!(
        output["plugin"]["manifest"]["artifact"]["path"],
        "dist/index.js"
    );
    assert!(output["plugin"]["manifest"]["artifact"]["sha256"].is_null());

    remove_dir_all(dir).expect("cleanup entry");
}

#[test]
fn plugin_manifest_plan_rejects_manifest_validation_errors() {
    let dir = make_temp_dir("failures");
    write(dir.join("plugin.wasm"), b"wasm").expect("wasm");

    let output = run_cli(&[
        "plugin-manifest".to_owned(),
        "parse".to_owned(),
        "--dir".to_owned(),
        dir.to_string_lossy().into_owned(),
        "--json".to_owned(),
        json!({
            "name": "bad-cli",
            "version": "1.0.0",
            "wasm": "plugin.wasm",
            "sdk": "*",
            "cli": { "command": "" }
        })
        .to_string(),
        "--plan-json".to_owned(),
    ]);
    assert_eq!(output.code, 2);
    assert!(
        output
            .stderr
            .contains("plugin.json: cli.command must be a non-empty string"),
        "{}",
        output.stderr
    );

    remove_dir_all(dir).expect("cleanup failures");
}

fn write_manifest(dir: &Path, manifest: &serde_json::Value) {
    write(dir.join("plugin.json"), manifest.to_string()).expect("write manifest");
}

#[test]
fn plugin_manifest_discover_plan_cli_matches_missing_roots_contract() {
    let root = make_temp_dir("discover-missing");
    let output = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "discover".to_owned(),
        "--scan-dir".to_owned(),
        root.join("missing-root").to_string_lossy().into_owned(),
        "--runtime-version".to_owned(),
        "1.0.0".to_owned(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(output["command"], "plugin-manifest");
    assert_eq!(output["kind"], "discover");
    assert_eq!(output["plugins"], json!([]));
    assert_eq!(output["warnings"], json!([]));

    remove_dir_all(root).expect("cleanup discover missing");
}

#[test]
fn plugin_manifest_discover_plan_cli_matches_registry_gates_and_sorting() {
    let root = make_temp_dir("discover-gates");
    let plugins_dir = root.join("plugins");
    create_dir_all(&plugins_dir).expect("plugins");

    write_entry_plugin(
        &plugins_dir,
        "registry-bad-sdk",
        serde_json::Map::from_iter([("sdk".to_owned(), json!(">=999.0.0"))]),
    );
    write_entry_plugin(
        &plugins_dir,
        "registry-legacy-ok",
        serde_json::Map::from_iter([("weight".to_owned(), json!(50))]),
    );
    write_entry_plugin(
        &plugins_dir,
        "registry-disabled-ok",
        serde_json::Map::from_iter([("weight".to_owned(), json!(70))]),
    );
    write(
        plugins_dir.join(".overrides.json"),
        br#"{"registry-disabled-ok":1}"#,
    )
    .expect("overrides");

    let output = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "discover".to_owned(),
        "--scan-dir".to_owned(),
        plugins_dir.to_string_lossy().into_owned(),
        "--disabled".to_owned(),
        "registry-disabled-ok".to_owned(),
        "--runtime-version".to_owned(),
        "1.0.0".to_owned(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(
        output["plugins"][0]["manifest"]["name"],
        "registry-disabled-ok"
    );
    assert_eq!(output["plugins"][0]["manifest"]["weight"], 1);
    assert_eq!(output["plugins"][0]["disabled"], true);
    assert_eq!(
        output["plugins"][1]["manifest"]["name"],
        "registry-legacy-ok"
    );
    let warning_text = output["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .map(|warning| warning.as_str().expect("warning text"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(warning_text.contains("requires maw SDK"), "{warning_text}");
    assert!(
        warning_text.contains("legacy plugin") || warning_text.contains("legacy plugins"),
        "{warning_text}"
    );

    remove_dir_all(root).expect("cleanup discover gates");
}

fn write_entry_plugin(
    root: &Path,
    name: &str,
    manifest: serde_json::Map<String, serde_json::Value>,
) {
    let dir = root.join(name);
    create_dir_all(&dir).expect("plugin dir");
    write(
        dir.join("index.ts"),
        format!(
            "export default async function {}() {{}}\n",
            name.replace('-', "_")
        ),
    )
    .expect("entry");
    let mut full_manifest = serde_json::Map::from_iter([
        ("name".to_owned(), json!(name)),
        ("version".to_owned(), json!("1.0.0")),
        ("sdk".to_owned(), json!("*")),
        ("target".to_owned(), json!("js")),
        ("entry".to_owned(), json!("index.ts")),
    ]);
    full_manifest.extend(manifest);
    write(
        dir.join("plugin.json"),
        serde_json::to_vec_pretty(&serde_json::Value::Object(full_manifest)).expect("json"),
    )
    .expect("manifest");
}
