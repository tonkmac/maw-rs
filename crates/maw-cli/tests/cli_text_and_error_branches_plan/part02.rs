fn write_ts_plugin(root: &Path, name: &str, manifest: serde_json::Map<String, serde_json::Value>) {
    let dir = root.join(name);
    create_dir_all(&dir).expect("plugin dir");
    write(
        dir.join("index.ts"),
        b"export default async function plugin() {}\n",
    )
    .expect("entry");
    write(dir.join("lib.ts"), b"export const answer = 42;\n").expect("module");
    let mut full_manifest = serde_json::Map::from_iter([
        ("name".to_owned(), json!(name)),
        ("version".to_owned(), json!("1.0.0")),
        ("sdk".to_owned(), json!("*")),
        ("target".to_owned(), json!("js")),
        ("entry".to_owned(), json!("index.ts")),
        (
            "module".to_owned(),
            json!({ "path": "./lib.ts", "exports": ["answer"] }),
        ),
    ]);
    full_manifest.extend(manifest);
    write(
        dir.join("plugin.json"),
        serde_json::to_vec_pretty(&serde_json::Value::Object(full_manifest)).expect("json"),
    )
    .expect("manifest");
}

fn write_wasm_plugin(root: &Path, name: &str) {
    let dir = root.join(name);
    create_dir_all(&dir).expect("plugin dir");
    write(dir.join("plugin.wasm"), b"wasm bytes").expect("wasm");
    write(
        dir.join("plugin.json"),
        json!({
            "name": name,
            "version": "1.0.0",
            "sdk": "*",
            "wasm": "plugin.wasm"
        })
        .to_string(),
    )
    .expect("manifest");
}
