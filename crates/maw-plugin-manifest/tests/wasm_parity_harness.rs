use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use maw_plugin_manifest::{
    hash_file, invoke_plugin, BunInvokeRuntime, ExtismWasmInvokeRuntime, InvokeContext,
    InvokeResult, InvokeSource, LoadedPlugin, LoadedPluginKind, MawWasmHost, PluginManifest,
};
use serde_json::Value;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn golden_parity_trivial_bun_and_wasm_outputs_match_in_isolated_maw_home() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf();
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wasm-parity/trivial");
    let metadata: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture.join("metadata.json")).expect("metadata"),
    )
    .expect("metadata json");
    assert_eq!(metadata["assemblyscript"], "0.27.31");
    assert_eq!(metadata["extismAsPdk"], "1.0.0");
    assert_eq!(
        hash_file(&fixture.join("plugin.wasm")).expect("wasm hash"),
        metadata["wasmSha256"].as_str().expect("sha")
    );

    let temp = temp_dir("wasm-parity");
    let isolated_home = temp.join("home");
    create_dir_all(&isolated_home).expect("isolated MAW_HOME");
    let old_maw_home = std::env::var_os("MAW_HOME");
    let old_plugins_dir = std::env::var_os("MAW_PLUGINS_DIR");
    std::env::set_var("MAW_HOME", &isolated_home);
    std::env::remove_var("MAW_PLUGINS_DIR");

    let ctx = InvokeContext {
        source: InvokeSource::Cli,
        args: vec!["alpha".to_owned(), "beta".to_owned()],
    };

    let bun_plugin = make_bun_plugin(&repo.join("examples/wasm-parity/trivial/bun"));
    let mut bun_runtime = BunInvokeRuntime::default();
    let bun = invoke_plugin(&bun_plugin, &ctx, &mut bun_runtime);

    let wasm_plugin = load_wasm_fixture(&fixture);
    let host = seeded_host(&fixture, &wasm_plugin);
    let mut wasm_runtime =
        ExtismWasmInvokeRuntime::default().with_host(wasm_plugin.manifest.name.clone(), host);
    let wasm = invoke_plugin(&wasm_plugin, &ctx, &mut wasm_runtime);

    restore_env("MAW_HOME", old_maw_home);
    restore_env("MAW_PLUGINS_DIR", old_plugins_dir);
    let _ = std::fs::remove_dir_all(temp);

    assert_eq!(capture(&bun), capture(&wasm));
}

fn seeded_host(fixture: &Path, plugin: &LoadedPlugin) -> MawWasmHost {
    let host_state: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture.join("host-state.json")).expect("host-state"),
    )
    .expect("host-state json");
    host_state["calls"].as_array().expect("calls").iter().fold(
        MawWasmHost::new(plugin),
        |host, call| {
            host.with_fake_response(
                call["name"].as_str().expect("fake name"),
                call["input"].as_str().expect("fake input"),
                call["output"].as_str().expect("fake output"),
            )
        },
    )
}

fn capture(result: &InvokeResult) -> Value {
    serde_json::json!({
        "stdout": result.output.as_deref().unwrap_or(""),
        "stderr": result.error.as_deref().unwrap_or(""),
        "result": { "ok": result.ok, "output": result.output, "error": result.error }
    })
}

fn make_bun_plugin(dir: &Path) -> LoadedPlugin {
    LoadedPlugin {
        manifest: manifest("trivial-parity"),
        dir: dir.to_path_buf(),
        wasm_path: PathBuf::new(),
        entry_path: Some(dir.join("index.ts")),
        wasm_export: "handle".to_owned(),
        kind: LoadedPluginKind::Ts,
        disabled: false,
    }
}

fn load_wasm_fixture(dir: &Path) -> LoadedPlugin {
    LoadedPlugin {
        manifest: manifest("trivial-parity"),
        dir: dir.to_path_buf(),
        wasm_path: dir.join("plugin.wasm"),
        entry_path: None,
        wasm_export: "handle".to_owned(),
        kind: LoadedPluginKind::Wasm,
        disabled: false,
    }
}

fn manifest(name: &str) -> PluginManifest {
    PluginManifest {
        name: name.to_owned(),
        version: "1.0.0".to_owned(),
        weight: None,
        tier: None,
        wasm: None,
        entry: None,
        entry_export: Some("handle".to_owned()),
        sdk: "*".to_owned(),
        cli: None,
        api: None,
        description: None,
        author: None,
        hooks: None,
        cron: None,
        module: None,
        transport: None,
        engine: None,
        target: None,
        capability_namespaces: None,
        capabilities: Some(Vec::new()),
        capability_warnings: Vec::new(),
        dependencies: None,
        artifact: None,
    }
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    if let Some(value) = value {
        std::env::set_var(key, value);
    } else {
        std::env::remove_var(key);
    }
}

fn temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maw-rs-{prefix}-{}-{stamp}", std::process::id()));
    create_dir_all(&path).expect("temp dir");
    path
}
