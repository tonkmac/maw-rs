use maw_plugin_scaffold::{build_manifest_json, copy_tree, validate_plugin_name, PluginLanguage};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn validate_plugin_name_accepts_simple_lowercase_name() {
    assert_eq!(validate_plugin_name("hello"), None);
}

#[test]
fn validate_plugin_name_accepts_name_with_hyphens_and_digits() {
    assert_eq!(validate_plugin_name("my-plugin-2"), None);
}

#[test]
fn validate_plugin_name_accepts_name_with_underscores() {
    assert_eq!(validate_plugin_name("my_plugin"), None);
}

#[test]
fn validate_plugin_name_rejects_empty_string() {
    assert!(validate_plugin_name("").is_some());
}

#[test]
fn validate_plugin_name_rejects_name_starting_with_digit() {
    assert!(validate_plugin_name("2plugin").is_some());
}

#[test]
fn validate_plugin_name_rejects_name_with_uppercase_letters() {
    assert!(validate_plugin_name("MyPlugin").is_some());
}

#[test]
fn validate_plugin_name_rejects_name_with_spaces() {
    assert!(validate_plugin_name("my plugin").is_some());
}

#[test]
fn copy_tree_copies_files_preserving_structure() {
    let root = unique_temp_dir("copy-structure");
    let src = root.join("src");
    let dest = root.join("copy");
    fs::create_dir_all(src.join("sub")).expect("create source subdir");
    fs::write(src.join("a.txt"), "hello").expect("write source file");
    fs::write(src.join("sub").join("b.txt"), "world").expect("write nested source file");

    copy_tree(&src, &dest).expect("copy tree succeeds");

    assert_eq!(
        fs::read_to_string(dest.join("a.txt")).expect("read copied file"),
        "hello"
    );
    assert_eq!(
        fs::read_to_string(dest.join("sub").join("b.txt")).expect("read nested copied file"),
        "world"
    );

    fs::remove_dir_all(root).ok();
}

#[test]
fn copy_tree_skips_target_directory() {
    let root = unique_temp_dir("copy-skip-target");
    let src = root.join("src");
    let dest = root.join("copy");
    fs::create_dir_all(src.join("target")).expect("create target dir");
    fs::write(src.join("keep.txt"), "yes").expect("write kept file");
    fs::write(src.join("target").join("artifact.wasm"), "binary").expect("write skipped artifact");

    copy_tree(&src, &dest).expect("copy tree succeeds");

    assert!(dest.join("keep.txt").exists());
    assert!(!dest.join("target").exists());

    fs::remove_dir_all(root).ok();
}

#[test]
fn copy_tree_skips_git_and_node_modules_entries() {
    let root = unique_temp_dir("copy-skip-extra");
    let src = root.join("src");
    let dest = root.join("copy");
    fs::create_dir_all(src.join(".git")).expect("create git dir");
    fs::create_dir_all(src.join("node_modules")).expect("create node_modules dir");
    fs::write(src.join(".git").join("config"), "secret").expect("write git file");
    fs::write(src.join("node_modules").join("pkg.js"), "pkg").expect("write module file");

    copy_tree(&src, &dest).expect("copy tree succeeds");

    assert!(!dest.join(".git").exists());
    assert!(!dest.join("node_modules").exists());

    fs::remove_dir_all(root).ok();
}

#[test]
fn rust_manifest_matches_scaffolded_plugin_json_contract() {
    let data = manifest("my-rust-plugin", PluginLanguage::Rust);

    assert_eq!(data["name"], "my-rust-plugin");
    assert_eq!(data["version"], "0.1.0");
    assert_eq!(data["sdk"], "^1.0.0");
    assert_eq!(
        data["wasm"],
        "./target/wasm32-unknown-unknown/release/my_rust_plugin.wasm"
    );
    assert_eq!(data["cli"]["command"], "my-rust-plugin");
    assert_eq!(data["api"]["path"], "/api/plugins/my-rust-plugin");
}

#[test]
fn assemblyscript_manifest_matches_scaffolded_plugin_json_contract() {
    let data = manifest("my-as-plugin", PluginLanguage::AssemblyScript);

    assert_eq!(data["name"], "my-as-plugin");
    assert_eq!(data["version"], "0.1.0");
    assert_eq!(data["sdk"], "^1.0.0");
    assert_eq!(data["wasm"], "./build/release.wasm");
    assert_eq!(data["cli"]["command"], "my-as-plugin");
    assert_eq!(data["api"]["path"], "/api/plugins/my-as-plugin");
}

#[test]
fn build_manifest_json_normalizes_underscores_to_hyphens_in_slug_fields() {
    let data = manifest("my_plugin", PluginLanguage::Rust);

    assert_eq!(data["name"], "my-plugin");
    assert!(data["wasm"]
        .as_str()
        .expect("wasm string")
        .contains("my_plugin.wasm"));
    assert_eq!(data["cli"]["command"], "my-plugin");
    assert_eq!(data["api"]["path"], "/api/plugins/my-plugin");
    assert_eq!(data["api"]["methods"], serde_json::json!(["GET", "POST"]));
}

#[test]
fn build_manifest_json_ends_with_newline() {
    assert!(build_manifest_json("my-plugin", PluginLanguage::Rust).ends_with('\n'));
}

fn manifest(name: &str, lang: PluginLanguage) -> Value {
    serde_json::from_str(&build_manifest_json(name, lang)).expect("valid manifest json")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "maw-rs-plugin-{label}-{}-{stamp}",
        std::process::id()
    ))
}
