use maw_cli::{dispatcher_status, DispatchKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maw-rs"))
}

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maw-rs-native-incubate-{name}-{stamp}"));
    fs::create_dir_all(&path).expect("temp dir");
    path
}

fn chmod_exec(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("chmod");
    }
}

fn write_fake_maw(bin_dir: &Path) {
    let maw = bin_dir.join("maw");
    fs::write(
        &maw,
        r#"#!/bin/sh
printf 'DELEGATED-MAW %s\n' "$*" >> "$MAW_INCUBATE_BUD_LOG"
exit 37
"#,
    )
    .expect("write fake maw");
    chmod_exec(&maw);
}

fn write_fake_tmux(bin_dir: &Path) {
    let tmux = bin_dir.join("tmux");
    fs::write(
        &tmux,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$MAW_INCUBATE_TMUX_LOG"
case "$1" in
  list-windows)
    printf 'widgets|||0|||main|||1|||/tmp\n'
    ;;
  send-keys)
    exit 0
    ;;
  capture-pane)
    printf '\n'
    ;;
  *)
    exit 0
    ;;
esac
"#,
    )
    .expect("write fake tmux");
    chmod_exec(&tmux);
}

fn seed_config(root: &Path) {
    let config = root.join("xdg-config").join("maw");
    fs::create_dir_all(&config).expect("config dir");
    fs::write(
        config.join("maw.config.json"),
        r#"{"node":"ci","oracle":"incubate-test"}"#,
    )
    .expect("seed config");
}

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    let bin_dir = root.join("bin");
    let home = root.join("home");
    let xdg_config = root.join("xdg-config");
    let xdg_data = root.join("xdg-data");
    let xdg_state = root.join("xdg-state");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&xdg_data).expect("xdg data");
    fs::create_dir_all(&xdg_state).expect("xdg state");

    Command::new(bin())
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("PATH", &bin_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env("XDG_DATA_HOME", &xdg_data)
        .env("XDG_STATE_HOME", &xdg_state)
        .env("TMUX", root.join("tmux-socket"))
        .env("MAW_JS_REF_DIR", "/nonexistent")
        .env("GHQ_ROOT", root.join("ghq/github.com"))
        .env("MAW_INCUBATE_BUD_LOG", root.join("bud.log"))
        .env("MAW_INCUBATE_TMUX_LOG", root.join("tmux.log"))
        .output()
        .expect("run maw-rs")
}

#[test]
fn native_incubate_dry_run_matches_committed_golden_and_is_hermetic() {
    let root = temp_dir("dry-run");
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_fake_maw(&bin_dir);
    seed_config(&root);

    let output = run(&root, &["incubate", "org/foo", "--dry-run", "--flash"]);

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.contains("Oracle scaffold plan"), "{stdout}");
    assert!(
        stdout.contains("[dry-run] would create repo: Soul-Brews-Studio/foo-oracle"),
        "{stdout}"
    );
    assert!(
        stdout.contains("[dry-run] would send \u{1b}[33m/incubate org/foo --flash\u{1b}[0m to foo"),
        "{stdout}"
    );
    assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
    assert!(
        !root.join("bud.log").exists(),
        "must not delegate to PATH maw"
    );
    assert!(
        !root.join("tmux.log").exists(),
        "dry-run must not touch tmux"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn native_incubate_dispatches_trigger_after_bud_and_guards_options() {
    let root = temp_dir("send");
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    write_fake_maw(&bin_dir);
    write_fake_tmux(&bin_dir);
    seed_config(&root);

    assert_eq!(dispatcher_status("incubate"), DispatchKind::Native);

    let output = run(
        &root,
        &["incubate", "org/widgets", "--contribute", "--dry-run"],
    );
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.contains("Oracle scaffold plan"), "{stdout}");
    assert!(
        stdout.contains(
            "[dry-run] would send \u{1b}[33m/incubate org/widgets --contribute\u{1b}[0m to widgets"
        ),
        "{stdout}"
    );
    assert!(
        !root.join("bud.log").exists(),
        "must not delegate to PATH maw"
    );
    assert!(
        !root.join("tmux.log").exists(),
        "dry-run must not touch tmux"
    );

    let bad = run(&root, &["incubate", "org/widgets", "--stem", "-bad"]);
    assert!(!bad.status.success());
    assert!(String::from_utf8(bad.stderr)
        .expect("stderr")
        .contains("--stem requires a value"));
    fs::remove_dir_all(root).expect("cleanup");
}
