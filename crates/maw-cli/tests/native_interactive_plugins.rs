use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use maw_cli::{dispatcher_status, DispatchKind};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maw-rs"))
}

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maw-rs-4c-{name}-{stamp}"));
    fs::create_dir_all(&path).expect("temp dir");
    path
}

fn run(args: &[&str], maw_home: &Path) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .env("MAW_HOME", maw_home)
        .env("MAW_JS_REF_DIR", "/nonexistent")
        .output()
        .expect("run maw-rs")
}

#[test]
fn interactive_plugin_commands_are_native_not_bun_fallback() {
    for command in ["init", "tmux", "view", "split"] {
        assert_eq!(
            dispatcher_status(command),
            DispatchKind::Native,
            "{command}"
        );
    }
}

#[test]
fn init_non_interactive_writes_maw_home_bounded_config_atomically() {
    let root = temp_dir("init-noninteractive");
    let output = run(
        &[
            "init",
            "--non-interactive",
            "--node",
            "nova-node",
            "--token",
            "test-token",
            "--federate",
            "--peer",
            "http://peer.example:3456",
            "--peer-name",
            "peer-one",
            "--federation-token",
            "feedface",
            "--force",
        ],
        &root,
    );

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config_path = root.join("config/maw.config.json");
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(config_path).expect("config body"))
            .expect("config json");
    assert_eq!(config["host"], "local");
    assert_eq!(config["node"], "nova-node");
    assert_eq!(config["env"]["CLAUDE_CODE_OAUTH_TOKEN"], "test-token");
    assert_eq!(config["namedPeers"][0]["name"], "peer-one");
    assert_eq!(config["namedPeers"][0]["url"], "http://peer.example:3456");
    assert_eq!(config["federationToken"], "feedface");
    assert!(String::from_utf8(output.stdout)
        .expect("stdout")
        .contains("Wrote"));
}

#[test]
fn init_refuses_existing_config_without_force_or_backup() {
    let root = temp_dir("init-refuse");
    let first = run(
        &["init", "--non-interactive", "--node", "nova", "--force"],
        &root,
    );
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = run(&["init", "--non-interactive", "--node", "nova"], &root);
    assert!(!second.status.success());
    assert!(String::from_utf8(second.stderr)
        .expect("stderr")
        .contains("Use --force to overwrite"));
}

#[test]
fn tmux_split_dry_run_is_capturable_without_live_fleet_pane() {
    let root = temp_dir("split-dry-run");
    let output = run(
        &[
            "split",
            "%isolated",
            "--vertical",
            "--pct",
            "25",
            "--cmd",
            "echo hi",
            "--dry-run",
        ],
        &root,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout"),
        "tmux split-window -v -l 25% -t %isolated -- echo hi\n"
    );
}

#[test]
fn init_interactive_wizard_uses_isolated_pty_when_script_is_available() {
    let probe = Command::new("script")
        .args(["-q", "-e", "-c", "true", "/dev/null"])
        .output();
    if !probe.is_ok_and(|output| output.status.success()) {
        eprintln!("skipping isolated PTY init test: GNU-style script(1) unavailable");
        return;
    }
    let root = temp_dir("init-pty");
    let mut child = Command::new("script")
        .args(["-q", "-e", "-c"])
        .arg(format!("{} init --force", bin().display()))
        .arg("/dev/null")
        .env("MAW_HOME", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn script pty");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"pty-node\n\nN\n")
        .expect("write answers");
    let output = child.wait_with_output().expect("wait script");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("config/maw.config.json")).expect("config body"),
    )
    .expect("config json");
    assert_eq!(config["node"], "pty-node");
}
