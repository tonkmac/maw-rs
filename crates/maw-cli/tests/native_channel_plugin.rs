use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const SECRET_VALUE: &str = "super-secret-channel-token";

fn channel_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maw-rs"))
}

fn channel_temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maw-rs-channel-{name}-{stamp}"));
    fs::create_dir_all(&path).expect("temp dir");
    path
}

fn channel_seed(root: &Path) -> PathBuf {
    let home = root.join("home");
    let repo = root.join("repo");
    fs::create_dir_all(home.join(".claude/channels/hermes-discord")).expect("channel dir");
    fs::create_dir_all(home.join(".claude/plugins/cache/claude-plugins-official/discord"))
        .expect("plugin cache");
    fs::create_dir_all(home.join("channels-state/hermes")).expect("state dir");
    fs::create_dir_all(&repo).expect("repo");
    fs::write(
        home.join(".claude/channels/hermes-discord/config.json"),
        format!(
            r#"{{
  "plugins": [
    {{
      "id": "plugin:discord@claude-plugins-official",
      "env": {{
        "DISCORD_STATE_DIR": "~/channels-state/hermes",
        "DISCORD_BOT_TOKEN": "{SECRET_VALUE}"
      }}
    }}
  ],
  "token_source": "pass:discord/hermes-token",
  "permissionMode": "relay"
}}
"#
        ),
    )
    .expect("channel config");
    fs::write(
        repo.join(".mcp.json"),
        r#"{"mcpServers":{"relay":{"command":"node","args":["server.js"]}}}"#,
    )
    .expect("mcp json");
    repo
}

fn channel_empty_repo(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    fs::create_dir_all(&repo).expect("repo");
    repo
}

fn channel_command(root: &Path, cwd: &Path, args: &[&str]) -> Output {
    fs::create_dir_all(root.join("maw-home")).expect("maw home");
    fs::create_dir_all(root.join("maw-plugins")).expect("maw plugins");
    Command::new(channel_bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", root.join("home"))
        .env("MAW_HOME", root.join("maw-home"))
        .env("MAW_PLUGINS_DIR", root.join("maw-plugins"))
        .env("MAW_JS_REF_DIR", "/nonexistent")
        .output()
        .expect("run maw-rs")
}

fn channel_assert_golden(root: &Path, cwd: &Path, args: &[&str], golden: &str) {
    let output = channel_command(root, cwd, args);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert_eq!(stdout, golden);
    assert!(!stdout.contains(SECRET_VALUE));
    assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
}

#[test]
fn channel_help_matches_committed_golden_without_ref_checkout() {
    let root = channel_temp_dir("help");
    let repo = channel_seed(&root);
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "help"],
        include_str!("fixtures/native-channel/channel-help.stdout"),
    );
}

#[test]
fn channel_ls_and_list_json_match_committed_goldens_and_redact_tokens() {
    let root = channel_temp_dir("list");
    let repo = channel_seed(&root);
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "ls", "-v"],
        include_str!("fixtures/native-channel/channel-ls.stdout"),
    );
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "list", "hermes-discord", "--json"],
        include_str!("fixtures/native-channel/channel-list-json.stdout"),
    );
}

#[test]
fn channel_providers_matches_committed_golden_with_custom_mcp_provider() {
    let root = channel_temp_dir("providers");
    let repo = channel_seed(&root);
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "providers"],
        include_str!("fixtures/native-channel/channel-providers.stdout"),
    );
}

#[test]
fn channel_test_matches_committed_golden_without_real_discord_or_pass() {
    let root = channel_temp_dir("test");
    let repo = channel_seed(&root);
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "test", "hermes-discord"],
        include_str!("fixtures/native-channel/channel-test.stdout"),
    );
}

#[test]
fn channel_add_and_rm_plugin_match_committed_goldens_and_archive_previous_config() {
    let root = channel_temp_dir("add-rm");
    let repo = channel_empty_repo(&root);
    channel_assert_golden(
        &root,
        &repo,
        &[
            "channel",
            "add",
            "hermes-discord",
            "discord",
            "--env",
            "DISCORD_BOT_TOKEN=super-secret-channel-token",
            "--pass",
            "discord/hermes-token",
        ],
        include_str!("fixtures/native-channel/channel-add.stdout"),
    );
    let config_path = root.join("home/.claude/channels/hermes-discord/config.json");
    let config = fs::read_to_string(&config_path).expect("config");
    assert!(
        config.contains(SECRET_VALUE),
        "secret is stored only in hermetic temp config"
    );

    channel_assert_golden(
        &root,
        &repo,
        &["channel", "rm", "hermes-discord", "discord"],
        include_str!("fixtures/native-channel/channel-rm-plugin.stdout"),
    );
    let after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config_path).expect("config after rm"))
            .expect("json");
    assert_eq!(after["plugins"].as_array().expect("plugins").len(), 0);
    let archives = fs::read_dir(root.join("home/.claude/channels/hermes-discord/archive"))
        .expect("archive dir")
        .count();
    assert!(archives >= 1, "rm preserves previous config in archive");
}

#[test]
fn channel_add_multiple_then_rm_all_preserves_maw_js_no_confirm_parity() {
    let root = channel_temp_dir("rm-all");
    let repo = channel_empty_repo(&root);
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "add", "hermes-discord", "fakechat"],
        include_str!("fixtures/native-channel/channel-add-fakechat.stdout"),
    );
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "add", "hermes-discord", "telegram"],
        include_str!("fixtures/native-channel/channel-add-telegram.stdout"),
    );
    channel_assert_golden(
        &root,
        &repo,
        &["channel", "remove", "hermes-discord"],
        include_str!("fixtures/native-channel/channel-rm-all.stdout"),
    );
    let after: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("home/.claude/channels/hermes-discord/config.json"))
            .expect("config after rm all"),
    )
    .expect("json");
    assert_eq!(after["plugins"].as_array().expect("plugins").len(), 0);
}

#[test]
fn channel_add_repo_mode_writes_repo_config_and_gitignore_only() {
    let root = channel_temp_dir("repo-mode");
    let repo = channel_empty_repo(&root);
    let repo_target = repo.join("repo-target");
    fs::create_dir_all(&repo_target).expect("repo target");

    let output = channel_command(
        &root,
        &repo,
        &[
            "channel",
            "add",
            "hermes-discord",
            "discord",
            "--repo",
            "repo-target",
            "--env",
            "DISCORD_BOT_TOKEN=super-secret-channel-token",
        ],
    );

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.contains("repo mode"));
    assert!(!stdout.contains(SECRET_VALUE));
    assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
    assert!(repo_target.join(".claude/channel.json").exists());
    assert!(fs::read_to_string(repo_target.join(".gitignore"))
        .expect("gitignore")
        .contains(".claude/.env"));
    assert!(!root
        .join("home/.claude/channels/hermes-discord/config.json")
        .exists());
}

#[test]
fn channel_number_67_guards_reject_traversal_and_flag_values_before_io() {
    let root = channel_temp_dir("guards");
    let repo = channel_seed(&root);

    for args in [
        ["channel", "test", "../secret"].as_slice(),
        ["channel", "test", "-bad"].as_slice(),
        ["channel", "ls", "hermes/discord"].as_slice(),
        ["channel", "providers", "extra"].as_slice(),
        ["channel", "add", "-bad", "discord"].as_slice(),
        ["channel", "add", "hermes-discord", "../discord"].as_slice(),
        [
            "channel",
            "add",
            "hermes-discord",
            "discord",
            "--repo",
            "../repo",
        ]
        .as_slice(),
        ["channel", "rm", "hermes/discord"].as_slice(),
    ] {
        let output = channel_command(&root, &repo, args);
        assert!(!output.status.success(), "args={args:?}");
    }
}

#[test]
fn channel_dispatch_registers_part120_native() {
    assert_eq!(
        maw_cli::dispatcher_status("channel"),
        maw_cli::DispatchKind::Native
    );
}
