const DISPATCH_120: &[DispatcherEntry] = &[DispatcherEntry { command: "channel", handler: Handler::Sync(channel_run_command) }];

const CHANNEL_HELP: &str = "usage: maw channel <subcommand> [args]\n\nsubcommands:\n  ls [oracle] [--json] [-v] list channels (all or for specific oracle)\n  add <oracle> <plugin>    add channel plugin to oracle\n  rm <oracle> <plugin>     remove channel plugin from oracle\n  providers                list available channel providers\n  setup <oracle>           interactive channel setup wizard\n  test <oracle>            test channel configuration\n  migrate --to-repo [...]  copy global ~/.claude/channels/<oracle>/config.json\n                           into each oracle's <repo>/.claude/channel.json\n                           ([oracle...] empty = all; --dry-run / --remove-global)\n\nshorthand: discord → plugin:discord@claude-plugins-official\ngithub: prefix → delegates to setup wizard";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct ChannelConfig {
    plugins: Vec<ChannelPlugin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_source: Option<String>,
    #[serde(rename = "permissionMode", skip_serializing_if = "Option::is_none")]
    permission_mode: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct ChannelPlugin {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone)]
struct ChannelAddArgs {
    oracle: String,
    plugin_id: String,
    repo_path: Option<std::path::PathBuf>,
    env: std::collections::BTreeMap<String, String>,
    pass_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChannelProvider {
    short_name: String,
    plugin_id: String,
    kind: &'static str,
}

fn channel_run_command(argv: &[String]) -> CliOutput {
    match channel_run(argv) {
        Ok(stdout) | Err((0, stdout)) => CliOutput { code: 0, stdout, stderr: String::new() },
        Err((code, message)) => CliOutput { code, stdout: String::new(), stderr: format!("{message}\n") },
    }
}

fn channel_run(argv: &[String]) -> Result<String, (i32, String)> {
    let sub = argv.first().map(|value| value.to_ascii_lowercase());
    match sub.as_deref() {
        Some("help" | "--help" | "-h") => Ok(format!("{CHANNEL_HELP}\n")),
        Some("ls" | "list") | None => channel_ls(&argv[1.min(argv.len())..]),
        Some("providers") => channel_providers(&argv[1..]),
        Some("test") => channel_test(&argv[1..]),
        Some("add") => channel_add(&argv[1..]),
        Some("rm" | "remove") => channel_rm(&argv[1..]),
        Some("setup" | "migrate") => Err((1, format!("channel: subcommand '{}' is not part of this native slice", sub.unwrap_or_default()))),
        Some(_) => Ok(channel_short_usage()),
    }
}

fn channel_short_usage() -> String {
    "usage: maw channel <add|rm|ls|providers|setup|test|migrate> [oracle] [plugin]\n\n  maw channel providers                          list available providers\n  maw channel setup hermes-discord discord       interactive wizard\n  maw channel setup myoracle github:org/repo     git channel wizard\n  maw channel add hermes-discord discord         quick register\n  maw channel add myoracle github:org/repo       git channel\n  maw channel rm hermes-discord discord          remove channel\n  maw channel ls                                 list all\n  maw channel test hermes-discord                verify connectivity\n  maw channel migrate --to-repo [oracle...]      global → repo (#1195)\n\n  maw wake <oracle> auto-injects --channels when config exists\n".to_owned()
}

fn channel_add(argv: &[String]) -> Result<String, (i32, String)> {
    use std::fmt::Write as _;

    let add_args = channel_parse_add(argv)?;
    let path = channel_config_path_for_add(&add_args.oracle, add_args.repo_path.as_deref());
    let mut config = channel_load_config_at(&path).unwrap_or_default();
    if config.plugins.iter().any(|plugin| plugin.id == add_args.plugin_id) {
        return Ok(format!("  \x1b[33m⚠\x1b[0m '{}' already registered for {}\n", add_args.plugin_id, add_args.oracle));
    }

    let plugin = channel_new_plugin(&add_args);
    if let Some(pass_key) = &add_args.pass_key { config.token_source = Some(format!("pass:{pass_key}")); }
    config.plugins.push(plugin.clone());
    channel_archive_existing_config(&path)?;
    channel_save_config_at(&path, &config)?;

    let mut stdout = String::new();
    if let Some(repo_path) = &add_args.repo_path {
        channel_save_repo_gitignore(repo_path)?;
        let _ = writeln!(stdout, "  \x1b[36m📁\x1b[0m repo mode — wrote {}/.claude/channel.json", repo_path.display());
    }
    let _ = writeln!(stdout, "  \x1b[32m✅\x1b[0m channel added: {} → {}", add_args.oracle, add_args.plugin_id);
    channel_push_added_env(&mut stdout, &plugin);
    channel_push_added_token(&mut stdout, &config);
    let _ = writeln!(stdout, "     next: \x1b[36mmaw wake {}\x1b[0m (channels auto-injected)", add_args.oracle);
    Ok(stdout)
}

fn channel_rm(argv: &[String]) -> Result<String, (i32, String)> {
    let (oracle, plugin) = channel_parse_rm(argv)?;
    let Some(mut config) = channel_load_oracle_config(&oracle) else {
        return Ok(format!("  \x1b[90mno channels for {oracle}\x1b[0m\n"));
    };
    if config.plugins.is_empty() { return Ok(format!("  \x1b[90mno channels for {oracle}\x1b[0m\n")); }

    let path = channel_oracle_config_path(&oracle);
    channel_archive_existing_config(&path)?;
    if let Some(plugin_id) = plugin {
        config.plugins.retain(|plugin| plugin.id != plugin_id);
        channel_save_config_at(&path, &config)?;
        Ok(format!("  \x1b[32m✓\x1b[0m removed {plugin_id} from {oracle}\n"))
    } else {
        config.plugins.clear();
        channel_save_config_at(&path, &config)?;
        Ok(format!("  \x1b[32m✓\x1b[0m removed all channels from {oracle}\n"))
    }
}

fn channel_parse_add(argv: &[String]) -> Result<ChannelAddArgs, (i32, String)> {
    if argv.len() < 2 {
        return Err((1, "usage: maw channel add <oracle> <plugin-id>".to_owned()));
    }
    let oracle = channel_validate_name("oracle", &argv[0])?;
    let plugin_id = channel_expand_plugin_id(&argv[1])?;
    let mut repo_path = None;
    let mut env = std::collections::BTreeMap::new();
    let mut pass_key = None;
    let mut index = 2;
    while index < argv.len() {
        match argv[index].as_str() {
            "--repo" => {
                let value = channel_take_flag_value(argv, index, "--repo")?;
                repo_path = Some(channel_validate_repo_path(value)?);
                index += 2;
            }
            "--env" => {
                let value = channel_take_flag_value(argv, index, "--env")?;
                let (key, env_value) = channel_validate_env_assignment(value)?;
                env.insert(key, env_value);
                index += 2;
            }
            "--pass" => {
                let value = channel_take_flag_value(argv, index, "--pass")?;
                pass_key = Some(channel_validate_pass_key(value)?);
                index += 2;
            }
            "--" => return Err((2, "channel: -- separator is not supported".to_owned())),
            other if other.starts_with('-') => return Err((2, format!("channel add: unknown flag {other}"))),
            other => return Err((2, format!("channel add: unexpected argument {other}"))),
        }
    }
    Ok(ChannelAddArgs { oracle, plugin_id, repo_path, env, pass_key })
}

fn channel_parse_rm(argv: &[String]) -> Result<(String, Option<String>), (i32, String)> {
    match argv {
        [] => Err((1, "usage: maw channel rm <oracle> [plugin-id]".to_owned())),
        [oracle] => Ok((channel_validate_name("oracle", oracle)?, None)),
        [oracle, plugin] => Ok((channel_validate_name("oracle", oracle)?, Some(channel_expand_plugin_id(plugin)?))),
        _ => Err((2, "channel rm accepts oracle and optional plugin only".to_owned())),
    }
}

fn channel_new_plugin(args: &ChannelAddArgs) -> ChannelPlugin {
    let mut env = args.env.clone();
    if args.plugin_id.contains("discord") && !env.contains_key("DISCORD_STATE_DIR") {
        let state_dir = if args.repo_path.is_some() { ".claude/channel-state".to_owned() } else { format!("~/.claude/channels/{}", args.oracle) };
        env.insert("DISCORD_STATE_DIR".to_owned(), state_dir);
    }
    ChannelPlugin { id: args.plugin_id.clone(), env: (!env.is_empty()).then_some(env) }
}

fn channel_push_added_env(stdout: &mut String, plugin: &ChannelPlugin) {
    use std::fmt::Write as _;

    if let Some(env) = &plugin.env {
        for (key, value) in env {
            let value = channel_display_env_value(key, value);
            let _ = writeln!(stdout, "     env: {key}={value}");
        }
    }
}

fn channel_push_added_token(stdout: &mut String, config: &ChannelConfig) {
    use std::fmt::Write as _;

    if let Some(token_source) = &config.token_source {
        let token_source = channel_display_token_source(token_source);
        let _ = writeln!(stdout, "     token: {token_source}");
    }
}

fn channel_expand_plugin_id(value: &str) -> Result<String, (i32, String)> {
    if value.starts_with("github:") {
        return Err((1, "channel add: github providers are handled by the setup slice".to_owned()));
    }
    channel_validate_plugin_id(value)?;
    if value.contains(':') || value.contains('@') { Ok(value.to_owned()) } else { Ok(format!("plugin:{value}@claude-plugins-official")) }
}

fn channel_validate_plugin_id(value: &str) -> Result<(), (i32, String)> {
    if value.is_empty()
        || value.trim() != value
        || value.starts_with('-')
        || value.contains("..")
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err((2, "channel: invalid plugin".to_owned()));
    }
    if !value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/' | '@')) {
        return Err((2, "channel: invalid plugin".to_owned()));
    }
    Ok(())
}

fn channel_take_flag_value<'a>(argv: &'a [String], index: usize, flag: &str) -> Result<&'a str, (i32, String)> {
    argv.get(index + 1)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| (2, format!("channel add: missing {flag} value")))
}

fn channel_validate_env_assignment(value: &str) -> Result<(String, String), (i32, String)> {
    let Some((key, env_value)) = value.split_once('=') else {
        return Err((2, "channel add: --env must be KEY=VALUE".to_owned()));
    };
    if key.is_empty()
        || key.starts_with('-')
        || key.chars().any(char::is_control)
        || !key.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err((2, "channel: invalid env key".to_owned()));
    }
    if env_value.chars().any(char::is_control) { return Err((2, "channel: invalid env value".to_owned())); }
    Ok((key.to_owned(), env_value.to_owned()))
}

fn channel_validate_pass_key(value: &str) -> Result<String, (i32, String)> {
    if value.is_empty()
        || value.trim() != value
        || value.starts_with('-')
        || value.contains("..")
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err((2, "channel: invalid pass key".to_owned()));
    }
    Ok(value.to_owned())
}

fn channel_validate_repo_path(value: &str) -> Result<std::path::PathBuf, (i32, String)> {
    let path = std::path::Path::new(value);
    if value.is_empty() || value.starts_with('-') || value.chars().any(char::is_control) {
        return Err((2, "channel: invalid repo path".to_owned()));
    }
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => return Err((2, "channel: invalid repo path".to_owned())),
            std::path::Component::Normal(name) if name.to_string_lossy().starts_with('-') => {
                return Err((2, "channel: invalid repo path".to_owned()));
            }
            _ => {}
        }
    }
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir().map(|cwd| cwd.join(path)).map_err(|error| (1, format!("channel: cannot resolve repo path: {error}")))
    }
}

fn channel_ls(argv: &[String]) -> Result<String, (i32, String)> {
    let (target, json, verbose) = channel_parse_ls(argv)?;
    if json { return Ok(channel_ls_json(target.as_deref())); }
    if let Some(target) = target { return Ok(channel_ls_one(&target, verbose)); }
    Ok(channel_ls_all(verbose))
}

fn channel_parse_ls(argv: &[String]) -> Result<(Option<String>, bool, bool), (i32, String)> {
    let mut target = None;
    let mut json = false;
    let mut verbose = false;
    for arg in argv {
        match arg.as_str() {
            "--json" => json = true,
            "--verbose" | "-v" => verbose = true,
            "--" => return Err((2, "channel: -- separator is not supported".to_owned())),
            value if value.starts_with('-') => return Err((2, format!("channel: unknown ls flag {value}"))),
            value => {
                if target.is_some() { return Err((2, "channel ls accepts at most one oracle".to_owned())); }
                target = Some(channel_validate_name("oracle", value)?);
            }
        }
    }
    Ok((target, json, verbose))
}

fn channel_providers(argv: &[String]) -> Result<String, (i32, String)> {
    use std::fmt::Write as _;

    channel_reject_extra_args("providers", argv)?;
    let providers = channel_get_providers();
    let mut stdout = format!("  \x1b[36;1mChannel Providers\x1b[0m ({} available)\n\n", providers.len());
    stdout.push_str("  Provider        Type       Plugin ID                                     Status\n");
    stdout.push_str("  ─────────────── ────────── ───────────────────────────────────────────── ──────────\n");
    for provider in providers {
        let status = if channel_is_plugin_installed(&provider.short_name) { "\x1b[32m✓ installed\x1b[0m" } else { "\x1b[90mnot installed\x1b[0m" };
        let _ = writeln!(stdout, "  {:<15} {:<10} {:<45} {status}", provider.short_name, provider.kind, provider.plugin_id);
    }
    stdout.push_str("\n  Install: \x1b[36m/plugin install <provider>@claude-plugins-official\x1b[0m\n");
    stdout.push_str("  Custom:  \x1b[36mmaw channel add <oracle> server:<name>\x1b[0m (for .mcp.json servers)\n");
    Ok(stdout)
}

fn channel_test(argv: &[String]) -> Result<String, (i32, String)> {
    let target = channel_parse_test(argv)?;
    let Some(config) = channel_load_oracle_config(&target) else {
        return Err((1, format!("  \x1b[31m✗\x1b[0m no channels for {target}")));
    };
    if config.plugins.is_empty() { return Err((1, format!("  \x1b[31m✗\x1b[0m no channels for {target}"))); }
    let env = channel_effective_env(&config);
    let mut stdout = format!("  \x1b[36;1mChannel Test: {target}\x1b[0m\n\n");
    for plugin in &config.plugins {
        stdout.push_str("  ");
        stdout.push_str(&plugin.id);
        stdout.push('\n');
        for check in channel_checks(plugin, &config, &env) {
            stdout.push_str("    ");
            stdout.push_str(&check);
            stdout.push('\n');
        }
    }
    Ok(stdout)
}

fn channel_parse_test(argv: &[String]) -> Result<String, (i32, String)> {
    match argv {
        [] => Err((1, "  usage: maw channel test <oracle>".to_owned())),
        [target] => channel_validate_name("oracle", target),
        _ => Err((2, "channel test accepts exactly one oracle".to_owned())),
    }
}

fn channel_reject_extra_args(subcommand: &str, argv: &[String]) -> Result<(), (i32, String)> {
    if argv.iter().any(|arg| arg == "--") { return Err((2, "channel: -- separator is not supported".to_owned())); }
    if let Some(arg) = argv.first() { return Err((2, format!("channel {subcommand}: unexpected argument {arg}"))); }
    Ok(())
}

fn channel_validate_name(label: &str, value: &str) -> Result<String, (i32, String)> {
    if value.is_empty()
        || value.trim() != value
        || value.starts_with('-')
        || value == "."
        || value == ".."
        || value.contains("..")
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err((2, format!("channel: invalid {label}")));
    }
    if !value.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')) {
        return Err((2, format!("channel: invalid {label}")));
    }
    Ok(value.to_owned())
}

fn channel_ls_json(target: Option<&str>) -> String {
    if let Some(target) = target {
        let config = channel_redacted_config(channel_load_oracle_config(target).unwrap_or_default());
        let mut value = serde_json::to_value(config).expect("channel config json");
        if let serde_json::Value::Object(map) = &mut value { map.insert("oracle".to_owned(), serde_json::json!(target)); }
        return format!("{}\n", serde_json::to_string_pretty(&value).expect("json"));
    }
    let oracles = channel_list_all_configs()
        .into_iter()
        .map(|(oracle, config)| serde_json::json!({ "oracle": oracle, "plugins": channel_redacted_config(config).plugins }))
        .collect::<Vec<_>>();
    format!("{}\n", serde_json::to_string_pretty(&serde_json::json!({ "oracles": oracles })).expect("json"))
}

fn channel_redacted_config(mut config: ChannelConfig) -> ChannelConfig {
    for plugin in &mut config.plugins {
        if let Some(env) = &mut plugin.env {
            for (key, value) in env.iter_mut() {
                if channel_is_secret_key(key) { "<redacted>".clone_into(value); }
            }
        }
    }
    if let Some(token_source) = &config.token_source {
        config.token_source = Some(channel_display_token_source(token_source));
    }
    config
}

fn channel_ls_one(target: &str, verbose: bool) -> String {
    let Some(config) = channel_load_oracle_config(target) else { return format!("  \x1b[90mno channels for {target}\x1b[0m\n"); };
    if config.plugins.is_empty() { return format!("  \x1b[90mno channels for {target}\x1b[0m\n"); }
    let mut stdout = format!("  \x1b[36;1m{target}\x1b[0m\n");
    for plugin in &config.plugins {
        stdout.push_str("    ");
        stdout.push_str(&plugin.id);
        stdout.push('\n');
        channel_push_plugin_env(&mut stdout, plugin, 6);
    }
    channel_push_token_source(&mut stdout, &config, 4);
    if verbose { channel_push_permission(&mut stdout, &config, 4); }
    stdout
}

fn channel_ls_all(verbose: bool) -> String {
    use std::fmt::Write as _;

    let all = channel_list_all_configs();
    if all.is_empty() {
        return "  \x1b[90mno oracles have channels configured\x1b[0m\n  add one: \x1b[36mmaw channel add <oracle> discord\x1b[0m\n".to_owned();
    }
    let mut stdout = format!("  \x1b[36;1mOracle{}Channel\x1b[0m\n", " ".repeat(24));
    let _ = writeln!(stdout, "  {}  {}", "─".repeat(30), "─".repeat(45));
    for (oracle, config) in &all {
        for plugin in &config.plugins {
            let _ = writeln!(stdout, "  {oracle:<30}  {}", plugin.id);
            if verbose {
                channel_push_plugin_env(&mut stdout, plugin, 32);
                channel_push_permission(&mut stdout, config, 32);
                channel_push_token_source(&mut stdout, config, 32);
            }
        }
    }
    let _ = writeln!(stdout, "\n  {} oracle(s) with channels", all.len());
    stdout
}

fn channel_push_plugin_env(stdout: &mut String, plugin: &ChannelPlugin, indent: usize) {
    use std::fmt::Write as _;

    if let Some(env) = &plugin.env {
        for (key, value) in env {
            let value = channel_display_env_value(key, value);
            let _ = writeln!(stdout, "{}\x1b[90m{key}={value}\x1b[0m", " ".repeat(indent));
        }
    }
}

fn channel_push_permission(stdout: &mut String, config: &ChannelConfig, indent: usize) {
    use std::fmt::Write as _;

    if let Some(mode) = &config.permission_mode {
        let _ = writeln!(stdout, "{}\x1b[90mpermissionMode: {mode}\x1b[0m", " ".repeat(indent));
    }
}

fn channel_push_token_source(stdout: &mut String, config: &ChannelConfig, indent: usize) {
    use std::fmt::Write as _;

    if let Some(token_source) = &config.token_source {
        let token_source = channel_display_token_source(token_source);
        let _ = writeln!(stdout, "{}\x1b[90mtoken: {token_source}\x1b[0m", " ".repeat(indent));
    }
}

fn channel_display_env_value(key: &str, value: &str) -> String {
    if channel_is_secret_key(key) { "<redacted>".to_owned() } else { value.to_owned() }
}

fn channel_display_token_source(value: &str) -> String {
    if matches!(value.split_once(':'), Some(("pass" | "env" | "keychain", _))) { value.to_owned() } else { "<redacted>".to_owned() }
}

fn channel_is_secret_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    ["TOKEN", "SECRET", "PASSWORD", "PASS", "PRIVATE_KEY"].iter().any(|needle| upper.contains(needle))
}

fn channel_get_providers() -> Vec<ChannelProvider> {
    let mut providers = vec![
        channel_provider("discord", "plugin:discord@claude-plugins-official", "chat"),
        channel_provider("telegram", "plugin:telegram@claude-plugins-official", "chat"),
        channel_provider("imessage", "plugin:imessage@claude-plugins-official", "chat"),
        channel_provider("fakechat", "plugin:fakechat@claude-plugins-official", "chat"),
    ];
    providers.extend(channel_custom_providers());
    providers
}

fn channel_provider(short_name: &str, plugin_id: &str, kind: &'static str) -> ChannelProvider {
    ChannelProvider { short_name: short_name.to_owned(), plugin_id: plugin_id.to_owned(), kind }
}

fn channel_custom_providers() -> Vec<ChannelProvider> {
    let mut providers = Vec::new();
    for path in [std::env::current_dir().ok().map(|cwd| cwd.join(".mcp.json")), Some(channel_home().join(".claude.json"))].into_iter().flatten() {
        let Some(json) = channel_read_json(&path) else { continue; };
        let Some(servers) = json.get("mcpServers").and_then(serde_json::Value::as_object) else { continue; };
        for name in servers.keys() {
            if channel_validate_name("server", name).is_ok() { providers.push(channel_provider(name, &format!("server:{name}"), "custom")); }
        }
    }
    providers
}

fn channel_is_plugin_installed(short_name: &str) -> bool {
    channel_home().join(".claude/plugins/cache/claude-plugins-official").join(short_name).exists()
}

fn channel_checks(plugin: &ChannelPlugin, config: &ChannelConfig, env: &std::collections::BTreeMap<String, String>) -> Vec<String> {
    let mut checks = Vec::new();
    if plugin.id.starts_with("plugin:") {
        let name = plugin.id.split(':').nth(1).and_then(|value| value.split('@').next()).unwrap_or_default();
        if channel_is_plugin_installed(name) { checks.push("\x1b[32m✓ plugin installed\x1b[0m".to_owned()); } else { checks.push("\x1b[31m✗ plugin not installed\x1b[0m".to_owned()); }
    }
    if let Some(dir) = env.get("DISCORD_STATE_DIR").or_else(|| plugin.env.as_ref().and_then(|map| map.get("DISCORD_STATE_DIR"))) {
        if std::path::Path::new(dir).exists() { checks.push("\x1b[32m✓ state dir exists\x1b[0m".to_owned()); } else { checks.push(format!("\x1b[31m✗ state dir missing: {dir}\x1b[0m")); }
    }
    if env.contains_key("DISCORD_BOT_TOKEN") || env.contains_key("TELEGRAM_BOT_TOKEN") { checks.push("\x1b[32m✓ token available\x1b[0m".to_owned()); } else if let Some(token_source) = &config.token_source { checks.push(format!("\x1b[32m✓ token source: {token_source}\x1b[0m")); } else { checks.push("\x1b[33m⚠ no token configured\x1b[0m".to_owned()); }
    checks
}

fn channel_effective_env(config: &ChannelConfig) -> std::collections::BTreeMap<String, String> {
    let mut env = std::collections::BTreeMap::new();
    for plugin in &config.plugins {
        if let Some(plugin_env) = &plugin.env { env.extend(plugin_env.clone()); }
    }
    let home = channel_home();
    for value in env.values_mut() {
        if let Some(stripped) = value.strip_prefix("~/") { *value = home.join(stripped).to_string_lossy().into_owned(); }
    }
    env
}


fn channel_config_path_for_add(oracle: &str, repo_path: Option<&std::path::Path>) -> std::path::PathBuf {
    repo_path.map_or_else(|| channel_oracle_config_path(oracle), channel_repo_config_path)
}

fn channel_oracle_config_path(oracle: &str) -> std::path::PathBuf {
    channel_channels_base().join(oracle).join("config.json")
}

fn channel_repo_config_path(repo_path: &std::path::Path) -> std::path::PathBuf {
    repo_path.join(".claude").join("channel.json")
}

fn channel_load_config_at(path: &std::path::Path) -> Option<ChannelConfig> {
    channel_read_json(path).and_then(|value| serde_json::from_value(value).ok())
}

fn channel_save_config_at(path: &std::path::Path, config: &ChannelConfig) -> Result<(), (i32, String)> {
    let json = serde_json::to_string_pretty(config).map_err(|error| (1, format!("channel: serialize config failed: {error}")))?;
    channel_atomic_write(path, &(json + "\n"))
}

fn channel_atomic_write(path: &std::path::Path, contents: &str) -> Result<(), (i32, String)> {
    let parent = path.parent().ok_or_else(|| (1, "channel: config path has no parent".to_owned()))?;
    std::fs::create_dir_all(parent).map_err(|error| (1, format!("channel: create config dir failed: {error}")))?;
    let tmp_path = parent.join(channel_tmp_file_name(path));
    std::fs::write(&tmp_path, contents).map_err(|error| (1, format!("channel: write temp config failed: {error}")))?;
    std::fs::rename(&tmp_path, path).map_err(|error| (1, format!("channel: rename temp config failed: {error}")))
}

fn channel_tmp_file_name(path: &std::path::Path) -> String {
    let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("config.json");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(".{name}.tmp.{}.{}", std::process::id(), nanos)
}

fn channel_archive_existing_config(path: &std::path::Path) -> Result<(), (i32, String)> {
    let Ok(contents) = std::fs::read_to_string(path) else { return Ok(()); };
    let parent = path.parent().ok_or_else(|| (1, "channel: config path has no parent".to_owned()))?;
    let archive_dir = parent.join("archive");
    let archive_name = channel_archive_file_name(path);
    channel_atomic_write(&archive_dir.join(archive_name), &contents)
}

fn channel_archive_file_name(path: &std::path::Path) -> String {
    let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("config.json");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{name}.{}.{}.bak", std::process::id(), nanos)
}

fn channel_save_repo_gitignore(repo_path: &std::path::Path) -> Result<(), (i32, String)> {
    let gitignore = repo_path.join(".gitignore");
    let entry = ".claude/.env";
    let existing = std::fs::read_to_string(&gitignore).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == entry) { return Ok(()); }
    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') { next.push('\n'); }
    next.push_str("\n# Channel bot token — never commit\n.claude/.env\n");
    channel_atomic_write(&gitignore, &next)
}

fn channel_list_all_configs() -> Vec<(String, ChannelConfig)> {
    let base = channel_channels_base();
    let Ok(entries) = std::fs::read_dir(base) else { return Vec::new(); };
    let mut configs = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) { continue; }
        let oracle = entry.file_name().to_string_lossy().into_owned();
        if channel_validate_name("oracle", &oracle).is_err() { continue; }
        if let Some(config) = channel_load_oracle_config(&oracle) {
            if !config.plugins.is_empty() { configs.push((oracle, config)); }
        }
    }
    configs.sort_by(|left, right| left.0.cmp(&right.0));
    configs
}

fn channel_load_oracle_config(oracle: &str) -> Option<ChannelConfig> {
    let path = channel_channels_base().join(oracle).join("config.json");
    channel_read_json(&path).and_then(|value| serde_json::from_value(value).ok())
}

fn channel_read_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn channel_channels_base() -> std::path::PathBuf { channel_home().join(".claude").join("channels") }

fn channel_home() -> std::path::PathBuf {
    std::env::var_os("HOME").map_or_else(|| std::path::PathBuf::from("."), std::path::PathBuf::from)
}
