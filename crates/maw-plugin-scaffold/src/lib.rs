//! Pure plugin scaffold helpers ported from maw-js
//! `src/commands/shared/plugin-create-scaffold.ts`.
//!
//! This crate ports the deterministic validation/manifest helpers plus the
//! template tree-copy plus Rust/AssemblyScript scaffold contracts from
//! `test/plugin-create.test.ts`.

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

/// Scaffold a Rust WASM plugin from a template directory.
///
/// Mirrors maw-js `scaffoldRust`: validates the template exists, copies the
/// template tree, rewrites `Cargo.toml` package name and SDK path, writes a
/// README, and emits `plugin.json`.
///
/// # Errors
///
/// Returns filesystem errors from template lookup, tree copy, reading/writing
/// `Cargo.toml`, README, or `plugin.json`.
pub fn scaffold_rust(
    name: &str,
    dest: impl AsRef<Path>,
    template_dir: impl AsRef<Path>,
    sdk_path: &str,
) -> io::Result<()> {
    let dest = dest.as_ref();
    let template_dir = template_dir.as_ref();
    if !template_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Rust template not found at {}", template_dir.display()),
        ));
    }

    copy_tree(template_dir, dest)?;

    let cargo_path = dest.join("Cargo.toml");
    let cargo = fs::read_to_string(&cargo_path)?;
    let cargo = rewrite_rust_cargo_toml(&cargo, name, sdk_path);
    fs::write(&cargo_path, cargo)?;

    fs::write(dest.join("README.md"), rust_readme(name, dest, sdk_path))?;
    fs::write(
        dest.join("plugin.json"),
        build_manifest_json(name, PluginLanguage::Rust),
    )?;
    Ok(())
}

/// Scaffold an `AssemblyScript` WASM plugin from a template directory.
///
/// Mirrors maw-js `scaffoldAs`: validates the template exists, copies the
/// template tree, rewrites package.json name when present, writes a README,
/// and emits `plugin.json`.
///
/// # Errors
///
/// Returns filesystem errors from template lookup, tree copy, reading/writing
/// `package.json`, README, or `plugin.json`, plus invalid package JSON.
pub fn scaffold_as(
    name: &str,
    dest: impl AsRef<Path>,
    template_dir: impl AsRef<Path>,
) -> io::Result<()> {
    let dest = dest.as_ref();
    let template_dir = template_dir.as_ref();
    if !template_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "AssemblyScript template not found at {}\n  The AS SDK is still being built — try again after the next maw update,\n  or check: https://github.com/Soul-Brews-Studio/maw-js",
                template_dir.display()
            ),
        ));
    }

    copy_tree(template_dir, dest)?;

    let package_path = dest.join("package.json");
    if package_path.exists() {
        let package = fs::read_to_string(&package_path)?;
        let package = rewrite_package_json_name(&package, name)?;
        fs::write(&package_path, package)?;
    }

    fs::write(dest.join("README.md"), as_readme(name, dest))?;
    fs::write(
        dest.join("plugin.json"),
        build_manifest_json(name, PluginLanguage::AssemblyScript),
    )?;
    Ok(())
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

fn rewrite_package_json_name(package: &str, name: &str) -> io::Result<String> {
    let mut value: Value = serde_json::from_str(package).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("package.json: invalid JSON: {error}"),
        )
    })?;
    match &mut value {
        Value::Object(object) => {
            object.insert("name".to_owned(), Value::String(name.to_owned()));
            serde_json::to_string_pretty(&value)
                .map(|text| format!("{text}\n"))
                .map_err(|error| {
                    io::Error::other(format!("package.json serialization failed: {error}"))
                })
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "package.json: must be a JSON object",
        )),
    }
}

fn as_readme(name: &str, dest: &Path) -> String {
    format!(
        r#"# {name}

A maw WASM command plugin (AssemblyScript).

## Build

```bash
cd "{}"
npm install
npm run build
```

Output: `build/{name}.wasm`

## Install

```bash
maw plugin install "{}"
```
"#,
        dest.display(),
        dest.display()
    )
}

fn rewrite_rust_cargo_toml(cargo: &str, name: &str, sdk_path: &str) -> String {
    let mut rewritten = cargo
        .lines()
        .map(|line| {
            if line.starts_with("name = ") {
                format!(r#"name = "{name}""#)
            } else if line.trim_start().starts_with("maw-plugin-sdk = { path = ") {
                format!(r#"maw-plugin-sdk = {{ path = "{sdk_path}" }}"#)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if cargo.ends_with('\n') {
        rewritten.push('\n');
    }
    rewritten
}

fn rust_readme(name: &str, dest: &Path, sdk_path: &str) -> String {
    let crate_name = name.replace('-', "_");
    format!(
        r#"# {name}

A maw WASM command plugin (Rust).

## Build

```bash
cd "{}"
cargo build --release --target wasm32-unknown-unknown
```

Output: `target/wasm32-unknown-unknown/release/{crate_name}.wasm`

## Install

```bash
maw plugin install "{}"
```

## SDK docs

See the SDK at `{sdk_path}` for available host functions:
`maw::print`, `maw::identity`, `maw::federation`, `maw::send`, `maw::fetch`.
"#,
        dest.display(),
        dest.display()
    )
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
