//! Pure plugin scaffold helpers ported from maw-js
//! `src/commands/shared/plugin-create-scaffold.ts`.
//!
//! This crate ports the deterministic validation/manifest helpers plus the
//! template tree-copy contract from `test/plugin-create.test.ts`.

use std::{fs, io, path::Path};

use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLanguage {
    Rust,
    AssemblyScript,
}

/// Copy a scaffold template tree while skipping build and package artifacts.
///
/// Mirrors maw-js `copyTree`: create the destination directory, recurse into
/// subdirectories, copy files, and skip `target`, `.git`, and `node_modules`
/// entries wherever they appear.
///
/// # Errors
///
/// Returns filesystem errors from reading the source tree, creating
/// directories, or copying files.
pub fn copy_tree(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
    copy_tree_inner(src.as_ref(), dest.as_ref())
}

/// Validate a plugin scaffold name.
///
/// Returns `None` for valid names and the maw-js error text for invalid names.
#[must_use]
pub fn validate_plugin_name(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("name is required".to_owned());
    }
    if !is_valid_plugin_name(name) {
        return Some(format!(
            "\"{name}\" is invalid — use lowercase letters, digits, - or _ (must start with a letter)"
        ));
    }
    None
}

/// Build plugin.json content for a scaffolded plugin.
///
/// Underscores are normalized to hyphens for slug fields, while Rust wasm crate
/// artifacts normalize hyphens to underscores like maw-js.
#[must_use]
pub fn build_manifest_json(name: &str, lang: PluginLanguage) -> String {
    let slug = name.replace('_', "-");
    let wasm_path = match lang {
        PluginLanguage::Rust => format!(
            "./target/wasm32-unknown-unknown/release/{}.wasm",
            name.replace('-', "_")
        ),
        PluginLanguage::AssemblyScript => "./build/release.wasm".to_owned(),
    };
    let type_name = match lang {
        PluginLanguage::Rust => "Rust",
        PluginLanguage::AssemblyScript => "AssemblyScript",
    };

    let mut manifest = Map::new();
    manifest.insert("name".to_owned(), json!(slug));
    manifest.insert("version".to_owned(), json!("0.1.0"));
    manifest.insert("wasm".to_owned(), json!(wasm_path));
    manifest.insert("sdk".to_owned(), json!("^1.0.0"));
    manifest.insert(
        "description".to_owned(),
        json!(format!("{type_name} plugin: {name}")),
    );
    manifest.insert("author".to_owned(), json!(""));
    manifest.insert(
        "cli".to_owned(),
        json!({ "command": slug, "help": format!("Invoke {name}") }),
    );
    manifest.insert(
        "api".to_owned(),
        json!({ "path": format!("/api/plugins/{slug}"), "methods": ["GET", "POST"] }),
    );

    let text = match serde_json::to_string_pretty(&Value::Object(manifest)) {
        Ok(text) => text,
        Err(error) => format!(r#"{{"error":"manifest serialization failed: {error}"}}"#),
    };
    format!("{text}\n")
}

fn copy_tree_inner(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if should_skip_entry(&file_name) {
            continue;
        }
        let source_path = entry.path();
        let dest_path = dest.join(file_name);
        if entry.file_type()?.is_dir() {
            copy_tree_inner(&source_path, &dest_path)?;
        } else {
            fs::copy(&source_path, &dest_path)?;
        }
    }
    Ok(())
}

fn should_skip_entry(name: &std::ffi::OsStr) -> bool {
    matches!(name.to_str(), Some("target" | ".git" | "node_modules"))
}

fn is_valid_plugin_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_'))
}
