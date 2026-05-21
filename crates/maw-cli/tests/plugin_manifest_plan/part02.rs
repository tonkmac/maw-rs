#[test]
fn plugin_manifest_invoke_plan_cli_dispatches_fake_ts_and_wasm_runtimes() {
    let root = make_temp_dir("invoke-runtime");
    let plugins_dir = root.join("plugins");
    create_dir_all(&plugins_dir).expect("plugins");
    write_invoke_ts_plugin(&plugins_dir, "ts-plug", serde_json::Map::new());
    write_invoke_wasm_plugin(&plugins_dir, "wasm-plug");

    let ts = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "invoke".to_owned(),
        "--scan-dir".to_owned(),
        plugins_dir.to_string_lossy().into_owned(),
        "--disabled".to_owned(),
        "other-plug".to_owned(),
        "--runtime-version".to_owned(),
        "1.2.3".to_owned(),
        "--use-cache".to_owned(),
        "--plugin".to_owned(),
        "ts-plug".to_owned(),
        "--source".to_owned(),
        "cli".to_owned(),
        "--arg".to_owned(),
        "a".to_owned(),
        "--arg".to_owned(),
        "b".to_owned(),
        "--fake-ts-output".to_owned(),
        "args=a|b".to_owned(),
        "--plan-json".to_owned(),
    ]));
    assert_eq!(
        ts["result"],
        json!({ "ok": true, "output": "args=a|b", "error": null })
    );
    assert_eq!(ts["runtime"]["tsCalls"], 1);
    assert_eq!(ts["runtime"]["wasmCalls"], 0);

    let wasm = json_output(&run_cli(&[
        "plugin-manifest".to_owned(),
        "invoke".to_owned(),
        "--scan-dir".to_owned(),
        plugins_dir.to_string_lossy().into_owned(),
        "--plugin".to_owned(),
        "wasm-plug".to_owned(),
        "--fake-wasm-output".to_owned(),
        "HELLO".to_owned(),
        "--plan-json".to_owned(),
    ]));
    assert_eq!(
        wasm["result"],
        json!({ "ok": true, "output": "HELLO", "error": null })
    );
    assert_eq!(wasm["runtime"]["tsCalls"], 0);
    assert_eq!(wasm["runtime"]["wasmCalls"], 1);
    assert_eq!(wasm["runtime"]["lastWasmBytesLen"], 10);

    remove_dir_all(root).expect("cleanup invoke runtime");
}

fn write_invoke_ts_plugin(
    root: &Path,
    name: &str,
    manifest: serde_json::Map<String, serde_json::Value>,
) {
    write_entry_plugin(root, name, manifest);
}

fn write_invoke_wasm_plugin(root: &Path, name: &str) {
    let dir = root.join(name);
    create_dir_all(&dir).expect("plugin dir");
    write(dir.join("plugin.wasm"), b"wasm bytes").expect("wasm");
    let full_manifest = serde_json::Map::from_iter([
        ("name".to_owned(), json!(name)),
        ("version".to_owned(), json!("1.0.0")),
        ("sdk".to_owned(), json!("*")),
        ("wasm".to_owned(), json!("plugin.wasm")),
    ]);
    write(
        dir.join("plugin.json"),
        serde_json::to_vec_pretty(&serde_json::Value::Object(full_manifest)).expect("json"),
    )
    .expect("manifest");
}
