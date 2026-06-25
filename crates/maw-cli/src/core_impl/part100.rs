const DISPATCH_100: &[DispatcherEntry] = &[DispatcherEntry {
    command: "ui",
    handler: Handler::Sync(ui_run_command),
}];

const UI_USAGE: &str = "usage: maw ui [--install] [--source] [--tunnel] [--dev] [--3d] [--version]";
const UI_TUNNEL_NOTE: &str = "ui --tunnel uses protected serve orchestration route /api/orchestration/workon; execution remains behind D2 auth";

const UI_FLAG_INSTALL: u8 = 1 << 0;
const UI_FLAG_SOURCE: u8 = 1 << 1;
const UI_FLAG_TUNNEL: u8 = 1 << 2;
const UI_FLAG_DEV: u8 = 1 << 3;
const UI_FLAG_THREE_D: u8 = 1 << 4;
const UI_FLAG_VERSION: u8 = 1 << 5;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct UiOptions {
    flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiPlan {
    mode: &'static str,
    dist_dir: String,
    source_dir: String,
    serve_url: String,
    commands: Vec<Vec<String>>,
    notes: Vec<String>,
}

fn ui_run_command(argv: &[String]) -> CliOutput {
    ui_run_command_with(argv, &ui_default_context())
}

fn ui_run_command_with(argv: &[String], context: &UiContext) -> CliOutput {
    match ui_run(argv, context) {
        Ok(stdout) => CliOutput {
            code: 0,
            stdout,
            stderr: String::new(),
        },
        Err(message) => CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("{message}\n"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiContext {
    cwd: std::path::PathBuf,
    data_dir: std::path::PathBuf,
    serve_url: String,
}

fn ui_default_context() -> UiContext {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    UiContext {
        data_dir: cwd.join(".maw").join("ui"),
        cwd,
        serve_url: "http://127.0.0.1:1977".to_owned(),
    }
}

fn ui_run(argv: &[String], context: &UiContext) -> Result<String, String> {
    let options = ui_parse_args(argv)?;
    if ui_has_flag(&options, UI_FLAG_VERSION) {
        return Ok(format!("maw ui {}\n", env!("CARGO_PKG_VERSION")));
    }
    let plan = ui_build_plan(&options, context)?;
    Ok(ui_render_plan(&plan))
}

fn ui_parse_args(argv: &[String]) -> Result<UiOptions, String> {
    let mut options = UiOptions::default();
    for arg in argv {
        match arg.as_str() {
            "--help" | "-h" | "help" => return Err(UI_USAGE.to_owned()),
            "--" => return Err("ui: -- separator is not allowed".to_owned()),
            "--install" => ui_set_flag(&mut options, UI_FLAG_INSTALL),
            "--source" => ui_set_flag(&mut options, UI_FLAG_SOURCE),
            "--tunnel" => ui_set_flag(&mut options, UI_FLAG_TUNNEL),
            "--dev" => ui_set_flag(&mut options, UI_FLAG_DEV),
            "--3d" => ui_set_flag(&mut options, UI_FLAG_THREE_D),
            "--version" | "-V" => ui_set_flag(&mut options, UI_FLAG_VERSION),
            value if value.starts_with('-') => return Err(ui_flag_like_value(value)),
            value => return Err(format!("ui: unexpected argument {value:?}\n  {UI_USAGE}")),
        }
    }
    Ok(options)
}

fn ui_set_flag(options: &mut UiOptions, flag: u8) {
    options.flags |= flag;
}

fn ui_has_flag(options: &UiOptions, flag: u8) -> bool {
    options.flags & flag != 0
}

fn ui_flag_like_value(value: &str) -> String {
    format!("ui: {value:?} is not a supported flag\n  {UI_USAGE}")
}

fn ui_build_plan(options: &UiOptions, context: &UiContext) -> Result<UiPlan, String> {
    ui_validate_context(context)?;
    let dist_dir = context.data_dir.join("dist");
    let source_dir = context.cwd.join("ui");
    let mut commands = Vec::new();
    let mut notes = Vec::new();
    let mode = ui_mode(options);
    if ui_has_flag(options, UI_FLAG_INSTALL) {
        commands.push(ui_command(&["install", ui_path_text(&dist_dir)?.as_str()]));
        notes.push("install validates the dist path; native port does not execute installers in CI".to_owned());
    }
    if ui_has_flag(options, UI_FLAG_SOURCE) {
        commands.push(ui_command(&["build-source", ui_path_text(&source_dir)?.as_str()]));
        notes.push("source build is planned only; git/npm process execution is deferred behind validated runner wiring".to_owned());
    }
    if ui_has_flag(options, UI_FLAG_DEV) {
        commands.push(ui_command(&["dev", ui_path_text(&source_dir)?.as_str()]));
    }
    if ui_has_flag(options, UI_FLAG_TUNNEL) {
        commands.push(ui_command(&["tunnel", context.serve_url.as_str()]));
        notes.push(UI_TUNNEL_NOTE.to_owned());
    }
    if ui_has_flag(options, UI_FLAG_THREE_D) {
        notes.push("3d UI mode requested".to_owned());
    }
    if commands.is_empty() {
        commands.push(ui_command(&["launch", ui_path_text(&dist_dir)?.as_str()]));
    }
    Ok(UiPlan {
        mode,
        dist_dir: ui_path_text(&dist_dir)?,
        source_dir: ui_path_text(&source_dir)?,
        serve_url: context.serve_url.clone(),
        commands,
        notes,
    })
}

fn ui_mode(options: &UiOptions) -> &'static str {
    if ui_has_flag(options, UI_FLAG_INSTALL) {
        "install"
    } else if ui_has_flag(options, UI_FLAG_SOURCE) {
        "source"
    } else if ui_has_flag(options, UI_FLAG_DEV) {
        "dev"
    } else if ui_has_flag(options, UI_FLAG_TUNNEL) {
        "tunnel"
    } else {
        "launch"
    }
}

fn ui_validate_context(context: &UiContext) -> Result<(), String> {
    ui_validate_path(&context.cwd, "cwd")?;
    ui_validate_path(&context.data_dir, "data dir")?;
    ui_validate_url(&context.serve_url)?;
    Ok(())
}

fn ui_validate_path(path: &std::path::Path, label: &str) -> Result<(), String> {
    let text = ui_path_text(path)?;
    if text == "--" || text.starts_with('-') || text.contains('\0') || text.contains('\n') || text.contains('\r') {
        return Err(format!("ui: {label} path is rejected"));
    }
    if path.components().any(ui_rejected_component) {
        return Err(format!("ui: {label} path contains a rejected segment"));
    }
    Ok(())
}

fn ui_path_text(path: &std::path::Path) -> Result<String, String> {
    let text = path.to_string_lossy();
    if text.is_empty() {
        return Err("ui: empty path is rejected".to_owned());
    }
    Ok(text.into_owned())
}

fn ui_rejected_component(component: std::path::Component<'_>) -> bool {
    matches!(component, std::path::Component::ParentDir)
        || component
            .as_os_str()
            .to_str()
            .is_some_and(|segment| segment == "--" || segment.starts_with('-') || segment.chars().any(char::is_control))
}

fn ui_validate_url(value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value || value == "--" || value.starts_with('-') || value.chars().any(char::is_control) {
        return Err("ui: serve url is rejected".to_owned());
    }
    let Some(rest) = value.strip_prefix("http://").or_else(|| value.strip_prefix("https://")) else {
        return Err("ui: serve url must be http(s)".to_owned());
    };
    if rest.is_empty() || rest.contains(' ') || rest.contains("..") {
        return Err("ui: serve url host/path is rejected".to_owned());
    }
    Ok(())
}

fn ui_command(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn ui_render_plan(plan: &UiPlan) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "ui plan:");
    let _ = writeln!(out, "  mode: {}", plan.mode);
    let _ = writeln!(out, "  dist: {}", plan.dist_dir);
    let _ = writeln!(out, "  source: {}", plan.source_dir);
    let _ = writeln!(out, "  serve: {}", plan.serve_url);
    for command in &plan.commands {
        let _ = writeln!(out, "  - {}", ui_shell_words(command));
    }
    for note in &plan.notes {
        let _ = writeln!(out, "  note: {note}");
    }
    out
}

fn ui_shell_words(words: &[String]) -> String {
    words.iter().map(|word| ui_shell_word(word)).collect::<Vec<_>>().join(" ")
}

fn ui_shell_word(word: &str) -> String {
    if word.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':' )) {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod ui_tests {
    use super::*;

    fn ui_strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn ui_context() -> UiContext {
        UiContext {
            cwd: std::path::PathBuf::from("/tmp/maw-ui-test/repo"),
            data_dir: std::path::PathBuf::from("/tmp/maw-ui-test/data/ui"),
            serve_url: "http://127.0.0.1:1977".to_owned(),
        }
    }

    #[test]
    fn ui_dispatch_registers_command() {
        assert_eq!(DISPATCH_100.len(), 1);
        assert_eq!(DISPATCH_100[0].command, "ui");
        assert_eq!(dispatcher_status("ui"), DispatchKind::Native);
    }

    #[test]
    fn ui_parse_flags_and_renders_plan_without_exec() {
        let output = ui_run(
            &ui_strings(&["--install", "--source", "--dev", "--3d"]),
            &ui_context(),
        )
        .expect("plan");
        assert!(output.contains("mode: install"));
        assert!(output.contains("install /tmp/maw-ui-test/data/ui/dist"));
        assert!(output.contains("build-source /tmp/maw-ui-test/repo/ui"));
        assert!(output.contains("dev /tmp/maw-ui-test/repo/ui"));
        assert!(output.contains("3d UI mode requested"));
    }

    #[test]
    fn ui_tunnel_is_stubbed_until_live_daemon_contract() {
        let output = ui_run(&ui_strings(&["--tunnel"]), &ui_context()).expect("plan");
        assert!(output.contains("tunnel http://127.0.0.1:1977"));
        assert!(output.contains("/api/orchestration/workon"));
    }

    #[test]
    fn ui_version_is_native_and_does_not_need_context() {
        let output = ui_run(&ui_strings(&["--version"]), &ui_context()).expect("version");
        assert!(output.starts_with("maw ui "));
    }

    #[test]
    fn ui_rejects_separator_unknown_flags_and_bad_context() {
        let err = ui_run(&ui_strings(&["--"]), &ui_context()).expect_err("separator");
        assert!(err.contains("separator"));
        let err = ui_run(&ui_strings(&["--bad"]), &ui_context()).expect_err("flag");
        assert!(err.contains("not a supported flag"));
        let mut context = ui_context();
        context.serve_url = "file:///tmp/app".to_owned();
        let err = ui_run(&ui_strings(&["--tunnel"]), &context).expect_err("url");
        assert!(err.contains("http(s)"));
        context.serve_url = "http://127.0.0.1:1977".to_owned();
        context.cwd = std::path::PathBuf::from("-bad");
        let err = ui_run(&ui_strings(&[]), &context).expect_err("path");
        assert!(err.contains("path"));
    }
}
