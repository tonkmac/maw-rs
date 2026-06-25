const DISPATCH_93: &[DispatcherEntry] = &[
    DispatcherEntry { command: "work", handler: Handler::Sync(work_run_command) },
    DispatcherEntry { command: "awake", handler: Handler::Sync(awake_run_command) },
    DispatcherEntry { command: "scaffold", handler: Handler::Sync(scaffold_run_command) },
    DispatcherEntry { command: "new", handler: Handler::Sync(new_run_command) },
    DispatcherEntry { command: "promote", handler: Handler::Sync(promote_run_command) },
    DispatcherEntry { command: "preflight", handler: Handler::Sync(preflight_run_command) },
    DispatcherEntry { command: "snapshots", handler: Handler::Sync(snapshots_run_command) },
];

const WORK_USAGE: &str = "usage: maw work <repo> [task] [--layout nested|legacy]";
const AWAKE_USAGE: &str = "usage: maw awake <name> [wake flags...]";
const SCAFFOLD_USAGE: &str = "usage: maw scaffold <name> [--rust|--as] [--dest <path>] [--dry-run]";
const NEW_USAGE: &str = "usage: maw new <name> [--rust|--as] [--dest <path>] [--dry-run]";
const PROMOTE_USAGE: &str = "usage: maw promote <target> [--base <branch>] [--branch <branch>] [--dry-run]";
const PREFLIGHT_USAGE: &str = "usage: maw preflight [path] [--json]";
const SNAPSHOTS_USAGE: &str = "usage: maw snapshots [list|create|show] [name] [--json]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaffoldLanguageNative {
    Rust,
    AssemblyScript,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaffoldOptionsNative {
    name: String,
    dest: std::path::PathBuf,
    language: ScaffoldLanguageNative,
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromoteOptionsNative {
    target: String,
    base: String,
    branch: Option<String>,
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreflightOptionsNative {
    path: std::path::PathBuf,
    json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SnapshotsActionNative {
    List,
    Create { name: String },
    Show { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotsOptionsNative {
    action: SnapshotsActionNative,
    json: bool,
}

fn work_run_command(argv: &[String]) -> CliOutput {
    if argv.iter().any(|arg| arg == "--") {
        return work_error("work: -- separator is not allowed");
    }
    if argv.is_empty() {
        return work_error(WORK_USAGE);
    }
    run_workon_command(argv)
}

fn work_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn awake_run_command(argv: &[String]) -> CliOutput {
    if argv.iter().any(|arg| arg == "--") {
        return awake_error("awake: -- separator is not allowed");
    }
    if argv.is_empty() {
        return awake_error(AWAKE_USAGE);
    }
    awake_dispatch_to_existing(argv)
}

fn awake_dispatch_to_existing(argv: &[String]) -> CliOutput {
    let mut forwarded = Vec::with_capacity(argv.len() + 1);
    forwarded.push("awaken".to_owned());
    forwarded.extend(argv.iter().cloned());
    run_cli(&forwarded)
}

fn awake_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn scaffold_run_command(argv: &[String]) -> CliOutput {
    match scaffold_parse_args(argv, SCAFFOLD_USAGE) {
        Ok(options) => match scaffold_apply(&options) {
            Ok(stdout) => CliOutput { code: 0, stdout, stderr: String::new() },
            Err(message) => scaffold_error(&message),
        },
        Err(message) => scaffold_error(&message),
    }
}

fn scaffold_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn scaffold_parse_args(argv: &[String], usage: &str) -> Result<ScaffoldOptionsNative, String> {
    let mut language = ScaffoldLanguageNative::Rust;
    let mut dest = None::<std::path::PathBuf>;
    let mut dry_run = false;
    let mut name = None::<String>;
    let mut index = 0_usize;
    while index < argv.len() {
        match argv[index].as_str() {
            "--help" | "-h" | "help" => return Err(usage.to_owned()),
            "--" => return Err("scaffold: -- separator is not allowed".to_owned()),
            "--rust" => language = ScaffoldLanguageNative::Rust,
            "--as" | "--assemblyscript" => language = ScaffoldLanguageNative::AssemblyScript,
            "--dry-run" => dry_run = true,
            "--dest" => { dest = Some(scaffold_path_value(argv, &mut index, "--dest")?); }
            value if value.starts_with("--dest=") => dest = Some(scaffold_validate_path(&value["--dest=".len()..])?),
            value if value.starts_with('-') => return Err(scaffold_flag_like(value)),
            value => scaffold_set_name(&mut name, value)?,
        }
        index += 1;
    }
    let name = name.ok_or_else(|| usage.to_owned())?;
    scaffold_validate_name(&name)?;
    let dest = dest.unwrap_or_else(|| std::path::PathBuf::from(&name));
    Ok(ScaffoldOptionsNative { name, dest, language, dry_run })
}

fn scaffold_set_name(slot: &mut Option<String>, value: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(SCAFFOLD_USAGE.to_owned());
    }
    if value.starts_with('-') {
        return Err(scaffold_flag_like(value));
    }
    *slot = Some(value.to_owned());
    Ok(())
}

fn scaffold_path_value(argv: &[String], index: &mut usize, flag: &str) -> Result<std::path::PathBuf, String> {
    let Some(value) = argv.get(*index + 1) else { return Err(format!("scaffold: {flag} requires a value")); };
    *index += 1;
    scaffold_validate_path(value)
}

fn scaffold_validate_name(name: &str) -> Result<(), String> {
    if name == "--" || name.starts_with('-') {
        return Err("scaffold name must not start with '-'".to_owned());
    }
    if let Some(error) = validate_plugin_name(name) {
        return Err(format!("scaffold: invalid plugin name: {error}"));
    }
    Ok(())
}

fn scaffold_validate_path(value: &str) -> Result<std::path::PathBuf, String> {
    if value.is_empty() || value.trim() != value || value == "--" || value.starts_with('-') || value.contains('\0') {
        return Err("scaffold path must be non-empty, unpadded, and not start with '-'".to_owned());
    }
    if value.split('/').any(|part| part == "..") {
        return Err("scaffold path must not contain .. segments".to_owned());
    }
    Ok(std::path::PathBuf::from(value))
}

fn scaffold_flag_like(value: &str) -> String {
    format!("\"{value}\" looks like a flag, not a scaffold name.\n  {SCAFFOLD_USAGE}")
}

fn scaffold_apply(options: &ScaffoldOptionsNative) -> Result<String, String> {
    scaffold_validate_destination(&options.dest)?;
    if options.dry_run {
        return Ok(scaffold_render_plan(options));
    }
    match options.language {
        ScaffoldLanguageNative::Rust => scaffold_write_rust(options)?,
        ScaffoldLanguageNative::AssemblyScript => scaffold_write_as(options)?,
    }
    Ok(scaffold_render_created(options))
}

fn scaffold_validate_destination(path: &std::path::Path) -> Result<(), String> {
    let display = path.display().to_string();
    scaffold_validate_path(&display)?;
    if path.exists() {
        return Err(format!("scaffold: destination exists: {}", path.display()));
    }
    Ok(())
}

fn scaffold_render_plan(options: &ScaffoldOptionsNative) -> String {
    format!("scaffold plan: create {} plugin {} at {}\n", scaffold_language_name(options.language), options.name, options.dest.display())
}

fn scaffold_render_created(options: &ScaffoldOptionsNative) -> String {
    format!("created {} plugin {} at {}\n", scaffold_language_name(options.language), options.name, options.dest.display())
}

fn scaffold_language_name(language: ScaffoldLanguageNative) -> &'static str {
    match language {
        ScaffoldLanguageNative::Rust => "rust",
        ScaffoldLanguageNative::AssemblyScript => "assemblyscript",
    }
}

fn scaffold_write_rust(options: &ScaffoldOptionsNative) -> Result<(), String> {
    std::fs::create_dir_all(options.dest.join("src")).map_err(|error| format!("scaffold: create rust dirs: {error}"))?;
    std::fs::write(options.dest.join("Cargo.toml"), scaffold_rust_cargo(&options.name)).map_err(|error| format!("scaffold: write Cargo.toml: {error}"))?;
    std::fs::write(options.dest.join("src/lib.rs"), scaffold_rust_lib()).map_err(|error| format!("scaffold: write src/lib.rs: {error}"))?;
    std::fs::write(options.dest.join("README.md"), scaffold_readme(&options.name, "Rust")).map_err(|error| format!("scaffold: write README.md: {error}"))?;
    std::fs::write(options.dest.join("plugin.json"), build_manifest_json(&options.name, ScaffoldLanguage::Rust)).map_err(|error| format!("scaffold: write plugin.json: {error}"))?;
    Ok(())
}

fn scaffold_write_as(options: &ScaffoldOptionsNative) -> Result<(), String> {
    std::fs::create_dir_all(options.dest.join("assembly")).map_err(|error| format!("scaffold: create as dirs: {error}"))?;
    std::fs::write(options.dest.join("package.json"), scaffold_as_package(&options.name)).map_err(|error| format!("scaffold: write package.json: {error}"))?;
    std::fs::write(options.dest.join("assembly/index.ts"), scaffold_as_index()).map_err(|error| format!("scaffold: write assembly/index.ts: {error}"))?;
    std::fs::write(options.dest.join("README.md"), scaffold_readme(&options.name, "AssemblyScript")).map_err(|error| format!("scaffold: write README.md: {error}"))?;
    std::fs::write(options.dest.join("plugin.json"), build_manifest_json(&options.name, ScaffoldLanguage::AssemblyScript)).map_err(|error| format!("scaffold: write plugin.json: {error}"))?;
    Ok(())
}

fn scaffold_rust_cargo(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n")
}

fn scaffold_rust_lib() -> &'static str {
    "#[no_mangle]\npub extern \"C\" fn maw_plugin_entry() -> i32 { 0 }\n"
}

fn scaffold_as_package(name: &str) -> String {
    format!("{{\n  \"name\": \"{name}\",\n  \"version\": \"0.1.0\",\n  \"scripts\": {{\"build\": \"asc assembly/index.ts --target release\"}}\n}}\n")
}

fn scaffold_as_index() -> &'static str {
    "export function mawPluginEntry(): i32 { return 0; }\n"
}

fn scaffold_readme(name: &str, language: &str) -> String {
    format!("# {name}\n\n{language} maw plugin scaffold.\n")
}

fn new_run_command(argv: &[String]) -> CliOutput {
    match new_parse_args(argv) {
        Ok(options) => match scaffold_apply(&options) {
            Ok(stdout) => CliOutput { code: 0, stdout: new_relabel_stdout(&stdout), stderr: String::new() },
            Err(message) => new_error(&message),
        },
        Err(message) => new_error(&message),
    }
}

fn new_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn new_parse_args(argv: &[String]) -> Result<ScaffoldOptionsNative, String> {
    scaffold_parse_args(argv, NEW_USAGE).map_err(|message| message.replace("scaffold", "new"))
}

fn new_relabel_stdout(stdout: &str) -> String {
    stdout.replace("scaffold plan:", "new plan:").replace("created", "created new")
}

fn promote_run_command(argv: &[String]) -> CliOutput {
    match promote_parse_args(argv) {
        Ok(options) => CliOutput { code: 0, stdout: promote_render_plan(&options), stderr: String::new() },
        Err(message) => promote_error(&message),
    }
}

fn promote_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn promote_parse_args(argv: &[String]) -> Result<PromoteOptionsNative, String> {
    let mut target = None::<String>;
    let mut base = "alpha".to_owned();
    let mut branch = None::<String>;
    let mut dry_run = false;
    let mut index = 0_usize;
    while index < argv.len() {
        match argv[index].as_str() {
            "--help" | "-h" | "help" => return Err(PROMOTE_USAGE.to_owned()),
            "--" => return Err("promote: -- separator is not allowed".to_owned()),
            "--dry-run" => dry_run = true,
            "--base" => base = promote_take_value(argv, &mut index, "--base")?,
            "--branch" => branch = Some(promote_take_value(argv, &mut index, "--branch")?),
            value if value.starts_with("--base=") => base = promote_validate_ref(&value["--base=".len()..], "base")?,
            value if value.starts_with("--branch=") => branch = Some(promote_validate_ref(&value["--branch=".len()..], "branch")?),
            value if value.starts_with('-') => return Err(promote_flag_like(value)),
            value => promote_set_target(&mut target, value)?,
        }
        index += 1;
    }
    Ok(PromoteOptionsNative { target: target.ok_or_else(|| PROMOTE_USAGE.to_owned())?, base, branch, dry_run })
}

fn promote_take_value(argv: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    let Some(value) = argv.get(*index + 1) else { return Err(format!("promote: {flag} requires a value")); };
    *index += 1;
    promote_validate_ref(value, flag)
}

fn promote_set_target(slot: &mut Option<String>, value: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(PROMOTE_USAGE.to_owned());
    }
    *slot = Some(promote_validate_ref(value, "target")?);
    Ok(())
}

fn promote_validate_ref(value: &str, label: &str) -> Result<String, String> {
    if value.is_empty() || value.trim() != value || value == "--" || value.starts_with('-') || value.contains("..") {
        return Err(format!("promote {label} must be non-empty, unpadded, and not start with '-'"));
    }
    if value.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err(format!("promote {label} must not contain whitespace or control characters"));
    }
    Ok(value.to_owned())
}

fn promote_flag_like(value: &str) -> String {
    format!("\"{value}\" looks like a flag, not a promote target.\n  {PROMOTE_USAGE}")
}

fn promote_render_plan(options: &PromoteOptionsNative) -> String {
    let branch = options.branch.as_deref().unwrap_or(&options.target);
    let mut out = String::new();
    let _ = writeln!(out, "promote plan:");
    let _ = writeln!(out, "  target: {}", options.target);
    let _ = writeln!(out, "  branch: {branch}");
    let _ = writeln!(out, "  base: {}", options.base);
    if options.dry_run {
        let _ = writeln!(out, "  mode: dry-run");
    }
    out.push_str("  note: native promote is plan-only until maw-js promote workflow parity is available.\n");
    out
}

fn preflight_run_command(argv: &[String]) -> CliOutput {
    match preflight_parse_args(argv).and_then(|options| preflight_run(&options)) {
        Ok(stdout) => CliOutput { code: 0, stdout, stderr: String::new() },
        Err(message) => preflight_error(&message),
    }
}

fn preflight_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn preflight_parse_args(argv: &[String]) -> Result<PreflightOptionsNative, String> {
    let mut path = None::<std::path::PathBuf>;
    let mut json = false;
    for arg in argv {
        match arg.as_str() {
            "--help" | "-h" | "help" => return Err(PREFLIGHT_USAGE.to_owned()),
            "--" => return Err("preflight: -- separator is not allowed".to_owned()),
            "--json" => json = true,
            value if value.starts_with('-') => return Err(preflight_flag_like(value)),
            value => preflight_set_path(&mut path, value)?,
        }
    }
    Ok(PreflightOptionsNative { path: path.unwrap_or_else(|| std::path::PathBuf::from(".")), json })
}

fn preflight_set_path(slot: &mut Option<std::path::PathBuf>, value: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(PREFLIGHT_USAGE.to_owned());
    }
    *slot = Some(preflight_validate_path(value)?);
    Ok(())
}

fn preflight_validate_path(value: &str) -> Result<std::path::PathBuf, String> {
    if value.is_empty() || value.trim() != value || value == "--" || value.starts_with('-') || value.contains('\0') {
        return Err("preflight path must be non-empty, unpadded, and not start with '-'".to_owned());
    }
    if value.split('/').any(|part| part == "..") {
        return Err("preflight path must not contain .. segments".to_owned());
    }
    Ok(std::path::PathBuf::from(value))
}

fn preflight_flag_like(value: &str) -> String {
    format!("\"{value}\" looks like a flag, not a preflight path.\n  {PREFLIGHT_USAGE}")
}

fn preflight_run(options: &PreflightOptionsNative) -> Result<String, String> {
    if !options.path.is_dir() {
        return Err(format!("preflight: not a directory: {}", options.path.display()));
    }
    let inside = preflight_git(&options.path, &["rev-parse", "--is-inside-work-tree"]).unwrap_or_default();
    let clean = preflight_git(&options.path, &["status", "--porcelain"]).unwrap_or_else(|_| "dirty".to_owned()).trim().is_empty();
    let ok = inside.trim() == "true" && clean;
    if options.json {
        return Ok(format!("{{\"command\":\"preflight\",\"path\":{},\"git\":{},\"clean\":{},\"ok\":{ok}}}\n", json_string(&options.path.display().to_string()), inside.trim() == "true", clean));
    }
    Ok(format!("preflight {}: git={} clean={} ok={}\n", options.path.display(), inside.trim() == "true", clean, ok))
}

fn preflight_git(path: &std::path::Path, args: &[&str]) -> Result<String, String> {
    preflight_validate_git_args(args)?;
    let output = std::process::Command::new("git").arg("-C").arg(path).args(args).output().map_err(|error| format!("preflight: git failed: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn preflight_validate_git_args(args: &[&str]) -> Result<(), String> {
    match args {
        ["rev-parse", "--is-inside-work-tree"] | ["status", "--porcelain"] => Ok(()),
        _ => Err("preflight: refused unexpected git argument shape".to_owned()),
    }
}

fn snapshots_run_command(argv: &[String]) -> CliOutput {
    match snapshots_parse_args(argv).and_then(|options| snapshots_run(&options)) {
        Ok(stdout) => CliOutput { code: 0, stdout, stderr: String::new() },
        Err(message) => snapshots_error(&message),
    }
}

fn snapshots_error(message: &str) -> CliOutput {
    CliOutput { code: 1, stdout: String::new(), stderr: format!("{message}\n") }
}

fn snapshots_parse_args(argv: &[String]) -> Result<SnapshotsOptionsNative, String> {
    let mut words = Vec::<String>::new();
    let mut json = false;
    for arg in argv {
        match arg.as_str() {
            "--help" | "-h" | "help" => return Err(SNAPSHOTS_USAGE.to_owned()),
            "--" => return Err("snapshots: -- separator is not allowed".to_owned()),
            "--json" => json = true,
            value if value.starts_with('-') => return Err(snapshots_flag_like(value)),
            value => words.push(snapshots_validate_name(value)?),
        }
    }
    let action = snapshots_action(&words)?;
    Ok(SnapshotsOptionsNative { action, json })
}

fn snapshots_action(words: &[String]) -> Result<SnapshotsActionNative, String> {
    match words {
        [] => Ok(SnapshotsActionNative::List),
        [one] if one == "list" => Ok(SnapshotsActionNative::List),
        [one] if one == "create" => Ok(SnapshotsActionNative::Create { name: snapshots_default_name() }),
        [one] => Ok(SnapshotsActionNative::Show { name: one.clone() }),
        [cmd, name] if cmd == "create" => Ok(SnapshotsActionNative::Create { name: name.clone() }),
        [cmd, name] if cmd == "show" => Ok(SnapshotsActionNative::Show { name: name.clone() }),
        _ => Err(SNAPSHOTS_USAGE.to_owned()),
    }
}

fn snapshots_validate_name(value: &str) -> Result<String, String> {
    if value.is_empty() || value.trim() != value || value == "--" || value.starts_with('-') || value.contains("..") {
        return Err("snapshots name must be non-empty, unpadded, and not start with '-'".to_owned());
    }
    if !value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')) {
        return Err("snapshots name must contain only ascii letters, digits, - or _".to_owned());
    }
    Ok(value.to_owned())
}

fn snapshots_flag_like(value: &str) -> String {
    format!("\"{value}\" looks like a flag, not a snapshot name.\n  {SNAPSHOTS_USAGE}")
}

fn snapshots_run(options: &SnapshotsOptionsNative) -> Result<String, String> {
    let dir = snapshots_dir();
    std::fs::create_dir_all(&dir).map_err(|error| format!("snapshots: create state dir: {error}"))?;
    match &options.action {
        SnapshotsActionNative::List => snapshots_list(&dir, options.json),
        SnapshotsActionNative::Create { name } => snapshots_create(&dir, name, options.json),
        SnapshotsActionNative::Show { name } => snapshots_show(&dir, name, options.json),
    }
}

fn snapshots_dir() -> std::path::PathBuf {
    maw_state_dir(&snapshots_xdg_env()).join("work-snapshots")
}

fn snapshots_xdg_env() -> MawXdgEnv {
    let home = std::env::var_os("HOME").map_or_else(|| std::path::PathBuf::from("."), std::path::PathBuf::from);
    let keys = ["MAW_HOME", "MAW_STATE_DIR", "MAW_XDG", "XDG_STATE_HOME"];
    MawXdgEnv::with_vars(home, keys.into_iter().filter_map(|key| std::env::var(key).ok().map(|value| (key, value))))
}

fn snapshots_default_name() -> String {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs());
    format!("snapshot-{seconds}")
}

fn snapshots_file(dir: &std::path::Path, name: &str) -> Result<std::path::PathBuf, String> {
    snapshots_validate_name(name)?;
    Ok(dir.join(format!("{name}.json")))
}

fn snapshots_list(dir: &std::path::Path, json: bool) -> Result<String, String> {
    let mut names = Vec::<String>::new();
    for entry in std::fs::read_dir(dir).map_err(|error| format!("snapshots: list: {error}"))?.flatten() {
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(std::ffi::OsStr::to_str) {
                names.push(stem.to_owned());
            }
        }
    }
    names.sort();
    if json {
        let body = names.iter().map(|name| json_string(name)).collect::<Vec<_>>().join(",");
        return Ok(format!("{{\"command\":\"snapshots\",\"snapshots\":[{body}]}}\n"));
    }
    Ok(if names.is_empty() { "no snapshots\n".to_owned() } else { format!("{}\n", names.join("\n")) })
}

fn snapshots_create(dir: &std::path::Path, name: &str, json: bool) -> Result<String, String> {
    let file = snapshots_file(dir, name)?;
    if file.exists() {
        return Err(format!("snapshots: snapshot exists: {name}"));
    }
    let cwd = std::env::current_dir().map_err(|error| format!("snapshots: cwd: {error}"))?;
    let body = format!("{{\"name\":{},\"cwd\":{},\"createdBy\":\"maw snapshots\"}}\n", json_string(name), json_string(&cwd.display().to_string()));
    std::fs::write(&file, &body).map_err(|error| format!("snapshots: write: {error}"))?;
    if json { Ok(body) } else { Ok(format!("created snapshot {name}\n")) }
}

fn snapshots_show(dir: &std::path::Path, name: &str, json: bool) -> Result<String, String> {
    let file = snapshots_file(dir, name)?;
    let body = std::fs::read_to_string(&file).map_err(|_| format!("snapshots: snapshot not found: {name}"))?;
    if json { Ok(body) } else { Ok(format!("{name}: {body}")) }
}

#[cfg(test)]
mod work_bundle_tests {
    use super::*;

    struct WorkBundleEnvGuard {
        root: std::path::PathBuf,
        saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl WorkBundleEnvGuard {
        fn work_new() -> Self {
            let keys = ["HOME", "XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"];
            let saved = keys.into_iter().map(|key| (key, std::env::var_os(key))).collect::<Vec<_>>();
            let root = std::env::temp_dir().join(format!("maw-work-bundle-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("home")).expect("home");
            std::fs::create_dir_all(root.join("state")).expect("state");
            std::env::set_var("HOME", root.join("home"));
            std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
            std::env::set_var("XDG_STATE_HOME", root.join("state"));
            std::env::set_var("XDG_DATA_HOME", root.join("data"));
            std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
            Self { root, saved }
        }
    }

    impl Drop for WorkBundleEnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.saved.drain(..) {
                if let Some(value) = value { std::env::set_var(key, value); } else { std::env::remove_var(key); }
            }
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn work_args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn work_dispatch_registers_seven_commands() {
        assert_eq!(DISPATCH_93.len(), 7);
        let commands = DISPATCH_93.iter().map(|entry| entry.command).collect::<Vec<_>>();
        assert_eq!(commands, ["work", "awake", "scaffold", "new", "promote", "preflight", "snapshots"]);
    }

    #[test]
    fn scaffold_and_new_create_hermetic_plugins() {
        let _lock = super::env_test_lock().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let env = WorkBundleEnvGuard::work_new();
        let rust_dest = env.root.join("hello-rust");
        let args = work_args(&["hello-rust", "--dest", rust_dest.to_str().expect("utf8")]);
        let out = scaffold_run_command(&args);
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(rust_dest.join("plugin.json").exists());
        let as_dest = env.root.join("hello-as");
        let args = work_args(&["hello-as", "--as", "--dest", as_dest.to_str().expect("utf8")]);
        let out = new_run_command(&args);
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(as_dest.join("assembly/index.ts").exists());
    }

    #[test]
    fn work_guards_reject_separator_and_leading_dash_values() {
        assert!(work_run_command(&work_args(&["--"])).stderr.contains("separator"));
        assert!(awake_run_command(&work_args(&["--"])).stderr.contains("separator"));
        assert!(scaffold_run_command(&work_args(&["-bad"])).stderr.contains("looks like a flag"));
        assert!(new_run_command(&work_args(&["-bad"])).stderr.contains("looks like a flag"));
        assert!(promote_run_command(&work_args(&["-bad"])).stderr.contains("looks like a flag"));
        assert!(preflight_run_command(&work_args(&["-bad"])).stderr.contains("looks like a flag"));
        assert!(snapshots_run_command(&work_args(&["-bad"])).stderr.contains("looks like a flag"));
    }

    #[test]
    fn promote_is_guarded_plan_only() {
        let out = promote_run_command(&work_args(&["agents/task", "--base", "alpha", "--dry-run"]));
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(out.stdout.contains("promote plan:"));
        assert!(out.stdout.contains("plan-only"));
    }

    #[test]
    fn snapshots_create_list_show_are_hermetic() {
        let _lock = super::env_test_lock().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _env = WorkBundleEnvGuard::work_new();
        let create = snapshots_run_command(&work_args(&["create", "alpha_snap"]));
        assert_eq!(create.code, 0, "{}", create.stderr);
        let list = snapshots_run_command(&work_args(&["list"]));
        assert!(list.stdout.contains("alpha_snap"));
        let show = snapshots_run_command(&work_args(&["show", "alpha_snap", "--json"]));
        assert!(show.stdout.contains("\"name\":\"alpha_snap\""));
    }

    #[test]
    fn preflight_json_reports_temp_git_repo_clean() {
        let _lock = super::env_test_lock().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let env = WorkBundleEnvGuard::work_new();
        std::process::Command::new("git").arg("init").arg(&env.root).output().expect("git init");
        let out = preflight_run_command(&work_args(&[env.root.to_str().expect("utf8"), "--json"]));
        assert_eq!(out.code, 0, "{}", out.stderr);
        assert!(out.stdout.contains("\"git\":true"));
    }
}
