#[derive(Debug, Clone, Default)]
struct SendArgs {
    target: String,
    text: String,
    inbox: Option<bool>,
    from: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct WakeArgs {
    target: String,
    task: Option<String>,
    from: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct HeyConfig {
    node: Option<String>,
    oracle: Option<String>,
    route: RouteConfig,
}

fn run_hey_async(args: Vec<String>) -> Pin<Box<dyn Future<Output = CliOutput> + Send>> {
    Box::pin(async move { run_send_like_async_impl("hey", &args).await })
}

fn run_send_async(args: Vec<String>) -> Pin<Box<dyn Future<Output = CliOutput> + Send>> {
    Box::pin(async move { run_send_like_async_impl("send", &args).await })
}

fn run_wake_async(args: Vec<String>) -> Pin<Box<dyn Future<Output = CliOutput> + Send>> {
    Box::pin(async move { run_wake_async_impl(&args).await })
}

async fn run_send_like_async_impl(command: &str, raw_args: &[String]) -> CliOutput {
    let fallback_env = format!("MAW_RS_{}_FALLBACK", command.to_ascii_uppercase());
    if std::env::var_os(fallback_env).is_some() {
        let mut fallback_argv = vec![command.to_owned()];
        fallback_argv.extend(raw_args.iter().cloned());
        return dispatch_bun_fallback(&fallback_argv, command);
    }

    let send_args = match parse_send_args(command, raw_args) {
        Ok(parsed) => parsed,
        Err(message) => return send_usage_error(command, &message),
    };
    let config = load_hey_config();
    let mut tmux = TmuxClient::local();
    let sessions = route_sessions_from_tmux(&mut tmux);
    match resolve_route_target(&send_args.target, &config.route, &sessions) {
        RouteResult::Local { target } | RouteResult::SelfNode { target } => send_local_message(
            command,
            &mut tmux,
            &target,
            &send_args.text,
            &config,
            send_args.from.as_deref(),
        ),
        RouteResult::Peer {
            peer_url,
            target,
            node: _,
        } => send_peer_message(command, &peer_url, &target, &send_args, &config).await,
        RouteResult::Error { detail, hint, .. } => CliOutput {
            code: 2,
            stdout: String::new(),
            stderr: if let Some(hint) = hint {
                format!("{command}: {detail}; {hint}\n")
            } else {
                format!("{command}: {detail}\n")
            },
        },
    }
}

fn parse_send_args(command: &str, argv: &[String]) -> Result<SendArgs, String> {
    let mut inbox = None;
    let mut from = None;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--inbox" => inbox = Some(true),
            "--no-inbox" => inbox = Some(false),
            "--from" => {
                let Some(value) = argv.get(index + 1) else {
                    return Err(format!("{command}: missing --from value"));
                };
                from = Some(value.clone());
                index += 1;
            }
            value if value.starts_with("--from=") => {
                from = Some(value["--from=".len()..].to_owned());
            }
            value if value.starts_with('-') => return Err(format!("{command}: unknown argument {value}")),
            value => positional.push(value.to_owned()),
        }
        index += 1;
    }
    if positional.len() < 2 {
        return Err(format!("{command}: target and message are required"));
    }
    Ok(SendArgs {
        target: positional[0].clone(),
        text: positional[1..].join(" "),
        inbox,
        from,
    })
}

fn send_usage_error(command: &str, message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{message}\nusage: maw-rs {command} <target> <message> [--inbox|--no-inbox] [--from <oracle:node>]\n"
        ),
    }
}

fn wake_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{message}\nusage: maw-rs wake <target> [--task <task>] [--from <oracle:node>]\n"
        ),
    }
}

fn send_local_message(
    command: &str,
    tmux: &mut TmuxClient<maw_tmux::CommandTmuxRunner>,
    target: &str,
    text: &str,
    config: &HeyConfig,
    from: Option<&str>,
) -> CliOutput {
    let outbound = format_local_hey_message(text, config, from);
    if let Err(error) = tmux.send_keys_literal(target, &outbound) {
        return CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("{command}: tmux send-keys failed: {error}\n"),
        };
    }
    if let Err(error) = tmux.send_enter(target) {
        return CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("{command}: tmux send-enter failed: {error}\n"),
        };
    }
    CliOutput {
        code: 0,
        stdout: format!("delivered {target}\n"),
        stderr: String::new(),
    }
}

async fn send_peer_message(
    command: &str,
    peer_url: &str,
    target: &str,
    args: &SendArgs,
    config: &HeyConfig,
) -> CliOutput {
    let from = match resolve_hey_wire_from(args.from.as_deref(), config) {
        Ok(from) => from,
        Err(message) => {
            return CliOutput {
                code: 2,
                stdout: String::new(),
                stderr: format!("{command}: {message}\n"),
            }
        }
    };
    let peer_key = match load_peer_key() {
        Ok(key) => key,
        Err(message) => {
            return CliOutput {
                code: 1,
                stdout: String::new(),
                stderr: format!("{command}: {message}\n"),
            }
        }
    };
    let client = match ReqwestHttpTransportIo::new(5_000) {
        Ok(client) => client,
        Err(message) => {
            return CliOutput {
                code: 1,
                stdout: String::new(),
                stderr: format!("{command}: {message}\n"),
            }
        }
    };
    let request = PeerSendRequest {
        peer_url: peer_url.to_owned(),
        target: target.to_owned(),
        text: args.text.clone(),
        inbox: args.inbox,
        from,
        peer_key,
        timestamp: i64::try_from(current_epoch_seconds()).unwrap_or(i64::MAX),
    };
    match client.send_peer(&request).await {
        Ok(response) => CliOutput {
            code: 0,
            stdout: format!(
                "{} {}\n",
                response.state.as_deref().unwrap_or("queued"),
                response.target.as_deref().unwrap_or(target)
            ),
            stderr: String::new(),
        },
        Err(message) => CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("{command}: {message}\n"),
        },
    }
}


async fn run_wake_async_impl(raw_args: &[String]) -> CliOutput {
    let wake_args = match parse_wake_args(raw_args) {
        Ok(parsed) => parsed,
        Err(message) => return wake_usage_error(&message),
    };
    let config = load_hey_config();
    let mut tmux = TmuxClient::local();
    let sessions = route_sessions_from_tmux(&mut tmux);
    match resolve_route_target(&wake_args.target, &config.route, &sessions) {
        RouteResult::Peer {
            peer_url,
            target,
            node: _,
        } => wake_peer_target(&peer_url, &target, &wake_args, &config).await,
        RouteResult::Local { .. } | RouteResult::SelfNode { .. } | RouteResult::Error { .. } => {
            let mut fallback_argv = vec!["wake".to_owned()];
            fallback_argv.extend(raw_args.iter().cloned());
            dispatch_bun_fallback(&fallback_argv, "wake")
        }
    }
}

fn parse_wake_args(argv: &[String]) -> Result<WakeArgs, String> {
    let mut from = None;
    let mut task = None;
    let mut positional = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--from" => {
                let Some(value) = argv.get(index + 1) else {
                    return Err("wake: missing --from value".to_owned());
                };
                from = Some(value.clone());
                index += 1;
            }
            value if value.starts_with("--from=") => {
                from = Some(value["--from=".len()..].to_owned());
            }
            "--task" => {
                let Some(value) = argv.get(index + 1) else {
                    return Err("wake: missing --task value".to_owned());
                };
                task = Some(value.clone());
                index += 1;
            }
            value if value.starts_with("--task=") => {
                task = Some(value["--task=".len()..].to_owned());
            }
            value if value.starts_with('-') => return Err(format!("wake: unknown argument {value}")),
            value => positional.push(value.to_owned()),
        }
        index += 1;
    }
    if positional.len() != 1 {
        return Err("wake: target is required".to_owned());
    }
    Ok(WakeArgs {
        target: positional[0].clone(),
        task,
        from,
    })
}

async fn wake_peer_target(
    peer_url: &str,
    target: &str,
    args: &WakeArgs,
    config: &HeyConfig,
) -> CliOutput {
    let from = match resolve_hey_wire_from(args.from.as_deref(), config) {
        Ok(from) => from,
        Err(message) => {
            return CliOutput {
                code: 2,
                stdout: String::new(),
                stderr: format!("wake: {message}\n"),
            }
        }
    };
    let peer_key = match load_peer_key() {
        Ok(key) => key,
        Err(message) => {
            return CliOutput {
                code: 1,
                stdout: String::new(),
                stderr: format!("wake: {message}\n"),
            }
        }
    };
    let client = match ReqwestHttpTransportIo::new(5_000) {
        Ok(client) => client,
        Err(message) => {
            return CliOutput {
                code: 1,
                stdout: String::new(),
                stderr: format!("wake: {message}\n"),
            }
        }
    };
    let request = PeerWakeRequest {
        peer_url: peer_url.to_owned(),
        target: target.to_owned(),
        task: args.task.clone(),
        from,
        peer_key,
        timestamp: i64::try_from(current_epoch_seconds()).unwrap_or(i64::MAX),
    };
    match client.wake_peer(&request).await {
        Ok(response) => CliOutput {
            code: 0,
            stdout: format!("woke {}\n", response.target.as_deref().unwrap_or(target)),
            stderr: String::new(),
        },
        Err(message) => CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("wake: {message}\n"),
        },
    }
}

fn resolve_hey_wire_from(explicit: Option<&str>, config: &HeyConfig) -> Result<String, String> {
    if let Some(value) = explicit {
        return validate_wire_from(value);
    }
    if let Ok(value) = std::env::var("MAW_SENDER") {
        return human_sender_to_wire_from(&value);
    }
    let node = config
        .node
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "cannot resolve sender identity; set MAW_SENDER or config node".to_owned())?;
    let oracle = config.oracle.as_deref().unwrap_or(DEFAULT_ORACLE);
    Ok(format!("{oracle}:{node}"))
}

fn validate_wire_from(value: &str) -> Result<String, String> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
        return Err("wire sender identity must be oracle:node".to_owned());
    }
    Ok(value.to_owned())
}

fn human_sender_to_wire_from(value: &str) -> Result<String, String> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
        return Err("MAW_SENDER must be node:oracle".to_owned());
    }
    Ok(format!("{}:{}", parts[1], parts[0]))
}

fn format_local_hey_message(text: &str, config: &HeyConfig, from: Option<&str>) -> String {
    if text.starts_with('/') || text.starts_with('[') {
        return text.to_owned();
    }
    let display = from.map_or_else(
        || {
            let node = config.node.as_deref().unwrap_or("local");
            let oracle = config.oracle.as_deref().unwrap_or(DEFAULT_ORACLE);
            format!("{node}:{oracle}")
        },
        ToOwned::to_owned,
    );
    format!("[{display}] {text}")
}

fn route_sessions_from_tmux(
    tmux: &mut TmuxClient<maw_tmux::CommandTmuxRunner>,
) -> Vec<RouteSession> {
    tmux.list_all()
        .into_iter()
        .map(|session| RouteSession {
            name: session.name,
            source: None,
            windows: session
                .windows
                .into_iter()
                .map(|window| RouteWindow {
                    index: window.index,
                    name: window.name,
                    active: window.active,
                })
                .collect(),
        })
        .collect()
}

fn load_hey_config() -> HeyConfig {
    let env = real_xdg_env();
    let path = maw_config_path(&env, &["maw.config.json"]);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return HeyConfig::default();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return HeyConfig::default();
    };
    let node = value
        .get("node")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    let oracle = value
        .get("oracle")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    let peers = value
        .get("peers")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let named_peers = parse_named_peers(value.get("namedPeers"));
    let agents = value
        .get("agents")
        .and_then(serde_json::Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| value.as_str().map(|node| (key.clone(), node.to_owned())))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    HeyConfig {
        node: node.clone(),
        oracle,
        route: RouteConfig {
            node,
            named_peers,
            peers,
            agents,
        },
    }
}

fn parse_named_peers(value: Option<&serde_json::Value>) -> Vec<RouteNamedPeer> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                Some(RouteNamedPeer {
                    name: item.get("name")?.as_str()?.to_owned(),
                    url: item.get("url")?.as_str()?.to_owned(),
                })
            })
            .collect(),
        Some(serde_json::Value::Object(map)) => map
            .iter()
            .filter_map(|(name, value)| {
                value.as_str().map(|url| RouteNamedPeer {
                    name: name.clone(),
                    url: url.to_owned(),
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn load_peer_key() -> Result<String, String> {
    if let Ok(value) = std::env::var("MAW_PEER_KEY") {
        if !value.is_empty() {
            return Ok(value);
        }
    }
    let env = real_xdg_env();
    let path = maw_state_path(&env, &["peer-key"]);
    if let Ok(raw) = std::fs::read_to_string(&path) {
        let key = raw.trim().to_owned();
        if !key.is_empty() {
            return Ok(key);
        }
    }
    let key = generate_peer_key()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create peer-key directory: {error}"))?;
    }
    write_peer_key_file(&path, &key)?;
    Ok(key)
}

fn generate_peer_key() -> Result<String, String> {
    let mut file = std::fs::File::open("/dev/urandom")
        .map_err(|error| format!("failed to open random peer key source: {error}"))?;
    let mut bytes = [0_u8; 32];
    std::io::Read::read_exact(&mut file, &mut bytes)
        .map_err(|error| format!("failed to read random peer key bytes: {error}"))?;
    Ok(hex_bytes(&bytes))
}

fn write_peer_key_file(path: &std::path::Path, key: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| format!("failed to write peer-key: {error}"))?;
        std::io::Write::write_all(&mut file, key.as_bytes())
            .map_err(|error| format!("failed to write peer-key: {error}"))?;
        std::io::Write::write_all(&mut file, b"\n")
            .map_err(|error| format!("failed to write peer-key: {error}"))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, format!("{key}\n"))
            .map_err(|error| format!("failed to write peer-key: {error}"))
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn real_xdg_env() -> MawXdgEnv {
    let home = std::env::var_os("HOME")
        .map_or_else(|| std::path::PathBuf::from("."), std::path::PathBuf::from);
    let vars = [
        "MAW_HOME",
        "MAW_CONFIG_DIR",
        "MAW_DATA_DIR",
        "MAW_STATE_DIR",
        "MAW_CACHE_DIR",
        "MAW_XDG",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
    ]
    .into_iter()
    .filter_map(|name| std::env::var(name).ok().map(|value| (name.to_owned(), value)));
    MawXdgEnv::with_vars(home, vars)
}
