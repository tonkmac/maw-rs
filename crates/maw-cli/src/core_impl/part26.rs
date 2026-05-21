fn parse_date_parts(value: &str) -> Result<DateParts, String> {
    let Some((date, time)) = value.split_once('T') else {
        return Err("calver: --now must use YYYY-M-DTHH:MM".to_owned());
    };
    let mut date_parts = date.split('-');
    let year = parse_i32_part(date_parts.next(), "year")?;
    let month = parse_u32_part(date_parts.next(), "month")?;
    let day = parse_u32_part(date_parts.next(), "day")?;
    if date_parts.next().is_some() {
        return Err("calver: --now date must use YYYY-M-D".to_owned());
    }

    let mut time_parts = time.split(':');
    let hour = parse_u32_part(time_parts.next(), "hour")?;
    let minute = parse_u32_part(time_parts.next(), "minute")?;
    if time_parts.next().is_some() {
        return Err("calver: --now time must use HH:MM".to_owned());
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return Err("calver: --now contains out-of-range date/time parts".to_owned());
    }
    Ok(DateParts {
        year,
        month,
        day,
        hour,
        minute,
    })
}

fn parse_i32_part(value: Option<&str>, name: &str) -> Result<i32, String> {
    let Some(value) = value else {
        return Err(format!("calver: missing {name} in --now"));
    };
    value
        .parse::<i32>()
        .map_err(|_| format!("calver: invalid {name} in --now"))
}

fn parse_u32_part(value: Option<&str>, name: &str) -> Result<u32, String> {
    let Some(value) = value else {
        return Err(format!("calver: missing {name} in --now"));
    };
    value
        .parse::<u32>()
        .map_err(|_| format!("calver: invalid {name} in --now"))
}

fn render_calver_plan_json(
    args: ComputeArgs,
    tags: &[String],
    package_version: &str,
    version: &str,
) -> String {
    let mut arg_fields = vec![
        format!("\"stable\":{}", args.stable),
        format!("\"now\":{}", render_date_parts_json(args.now)),
    ];
    if let Some(channel) = args.channel {
        arg_fields.push(format!("\"channel\":{}", json_string(channel.as_str())));
    }
    format!(
        "{{\"command\":\"calver\",\"args\":{{{}}},\"tags\":{},\"packageVersion\":{},\"version\":{}}}\n",
        arg_fields.join(","),
        json_string_array(tags),
        json_string(package_version),
        json_string(version)
    )
}

fn render_date_parts_json(now: DateParts) -> String {
    format!(
        "{{\"year\":{},\"month\":{},\"day\":{},\"hour\":{},\"minute\":{}}}",
        now.year, now.month, now.day, now.hour, now.minute
    )
}

fn run_normalize_plan(argv: &[String]) -> CliOutput {
    if matches!(argv.first().map(String::as_str), Some("constants")) {
        return run_normalize_constants_plan(&argv[1..]);
    }

    let plan_json = argv.iter().any(|arg| arg == "--plan-json");
    let target = argv.iter().find(|arg| arg.as_str() != "--plan-json");
    let Some(target) = target else {
        return CliOutput {
            code: 2,
            stdout: String::new(),
            stderr:
                "normalize: expected <target>\nusage: maw-rs normalize <target> [--plan-json]\nusage: maw-rs normalize constants [--plan-json]\n"
                    .to_owned(),
        };
    };
    let normalized = normalize_target(target);
    CliOutput {
        code: 0,
        stdout: if plan_json {
            format!(
                "{{\"command\":\"normalize\",\"input\":{},\"normalized\":{}}}\n",
                json_string(target),
                json_string(&normalized)
            )
        } else {
            format!("{normalized}\n")
        },
        stderr: String::new(),
    }
}

fn run_normalize_constants_plan(argv: &[String]) -> CliOutput {
    let mut plan_json = false;
    for arg in argv {
        match arg.as_str() {
            "--plan-json" => plan_json = true,
            _ => {
                return normalize_constants_usage_error(&format!(
                    "normalize constants: unknown argument {arg}"
                ))
            }
        }
    }

    CliOutput {
        code: 0,
        stdout: if plan_json {
            render_normalize_constants_json()
        } else {
            "normalize constants steps=trim,strip-trailing-slashes,strip-trailing-dot-git-until-stable\n"
                .to_owned()
        },
        stderr: String::new(),
    }
}

fn render_normalize_constants_json() -> String {
    concat!(
        "{\"command\":\"normalize\",\"kind\":\"constants\",",
        "\"steps\":[\"trim\",\"strip-trailing-slashes\",\"strip-trailing-dot-git-until-stable\"],",
        "\"preserves\":[\"interior characters\",\"case\",\"suffix text named .git without slash-dot\"],",
        "\"emptyBehavior\":{\"empty\":\"empty\",\"whitespaceOnly\":\"empty\"},",
        "\"fixtureCount\":12}\n"
    )
    .to_owned()
}

fn normalize_constants_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\nusage: maw-rs normalize constants [--plan-json]\n"),
    }
}

fn run_resolve_plan(argv: &[String]) -> CliOutput {
    if matches!(argv.first().map(String::as_str), Some("constants")) {
        return run_resolve_constants_plan(&argv[1..]);
    }

    let plan_json = argv.iter().any(|arg| arg == "--plan-json");
    let mut mode = "by-name".to_owned();
    let mut positionals = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--plan-json" => {}
            "--mode" => {
                let Some(value) = argv.get(index + 1) else {
                    return resolve_usage_error("resolve: missing --mode value");
                };
                mode.clone_from(value);
                index += 1;
            }
            arg => positionals.push(arg.to_owned()),
        }
        index += 1;
    }

    if positionals.len() < 2 {
        return resolve_usage_error("resolve: expected <target> and at least one item");
    }
    let target = &positionals[0];
    let items = &positionals[1..];
    let result = match mode.as_str() {
        "by-name" | "byName" => resolve_by_name(target, items, ResolveOptions::default()),
        "session" => resolve_session_target(target, items),
        "worktree" => resolve_worktree_target(target, items),
        _ => return resolve_usage_error("resolve: unknown --mode"),
    };
    let stdout = if plan_json {
        render_resolve_plan_json(&mode, target, result)
    } else {
        render_resolve_plan_text(&mode, target, result)
    };
    CliOutput {
        code: 0,
        stdout,
        stderr: String::new(),
    }
}

fn resolve_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\nusage: maw-rs resolve --mode <by-name|session|worktree> <target> <item...> [--plan-json]\nusage: maw-rs resolve constants [--plan-json]\n"),
    }
}

fn run_resolve_constants_plan(argv: &[String]) -> CliOutput {
    let mut plan_json = false;
    for arg in argv {
        match arg.as_str() {
            "--plan-json" => plan_json = true,
            _ => {
                return resolve_constants_usage_error(&format!(
                    "resolve constants: unknown argument {arg}"
                ))
            }
        }
    }

    CliOutput {
        code: 0,
        stdout: if plan_json {
            render_resolve_constants_json()
        } else {
            "resolve constants modes=by-name,session,worktree results=exact,fuzzy,ambiguous,none\n"
                .to_owned()
        },
        stderr: String::new(),
    }
}

fn render_resolve_constants_json() -> String {
    concat!(
        "{\"command\":\"resolve\",\"kind\":\"constants\",",
        "\"modes\":[\"by-name\",\"session\",\"worktree\"],\"modeAliases\":{\"byName\":\"by-name\"},",
        "\"resultKinds\":[\"exact\",\"fuzzy\",\"ambiguous\",\"none\"],",
        "\"matchLadder\":[\"trim-lowercase-target\",\"case-insensitive-exact\",\"suffix-segment\",\"prefix-or-middle-segment\",\"substring-hints-only\"],",
        "\"modeRules\":{\"session\":{\"fleetSessions\":true,\"numericPrefixBlocksPrefixMiddle\":true},\"worktree\":{\"fleetSessions\":false,\"numericPrefixesAreSequenceCounters\":true},\"by-name\":{\"fleetSessions\":false}},",
        "\"noneBehavior\":{\"emptyTarget\":\"none-no-hints\",\"substringFallback\":\"none-with-hints-never-fuzzy\"},",
        "\"fixtureCounts\":{\"total\":16,\"byName\":12,\"session\":3,\"worktree\":1,\"exact\":2,\"fuzzy\":7,\"ambiguous\":3,\"none\":4}}\n"
    )
    .to_owned()
}

fn resolve_constants_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\nusage: maw-rs resolve constants [--plan-json]\n"),
    }
}

fn render_resolve_plan_json(mode: &str, target: &str, result: ResolveResult<String>) -> String {
    let mut fields = vec![
        "\"command\":\"resolve\"".to_owned(),
        format!("\"mode\":{}", json_string(mode)),
        format!("\"target\":{}", json_string(target)),
    ];
    match result {
        ResolveResult::Exact { matched } => {
            fields.push("\"kind\":\"exact\"".to_owned());
            fields.push(format!("\"match\":{}", json_string(&matched)));
        }
        ResolveResult::Fuzzy { matched } => {
            fields.push("\"kind\":\"fuzzy\"".to_owned());
            fields.push(format!("\"match\":{}", json_string(&matched)));
        }
        ResolveResult::Ambiguous { candidates } => {
            fields.push("\"kind\":\"ambiguous\"".to_owned());
            fields.push(format!("\"candidates\":{}", json_string_array(&candidates)));
        }
        ResolveResult::None { hints } => {
            fields.push("\"kind\":\"none\"".to_owned());
            if let Some(hints) = hints {
                fields.push(format!("\"hints\":{}", json_string_array(&hints)));
            }
        }
    }
    format!("{{{}}}\n", fields.join(","))
}

fn render_resolve_plan_text(mode: &str, target: &str, result: ResolveResult<String>) -> String {
    match result {
        ResolveResult::Exact { matched } => {
            format!("resolve {mode} {target}: exact {matched}\n")
        }
        ResolveResult::Fuzzy { matched } => {
            format!("resolve {mode} {target}: fuzzy {matched}\n")
        }
        ResolveResult::Ambiguous { candidates } => {
            format!(
                "resolve {mode} {target}: ambiguous {}\n",
                candidates.join(", ")
            )
        }
        ResolveResult::None { hints } => hints.map_or_else(
            || format!("resolve {mode} {target}: none\n"),
            |hints| format!("resolve {mode} {target}: none hints={}\n", hints.join(", ")),
        ),
    }
}

fn json_string_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_str_array(values: &[&str]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_string(value))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn usage_ok() -> CliOutput {
    CliOutput {
        code: 0,
        stdout: usage_text(),
        stderr: String::new(),
    }
}

fn usage_text() -> String {
    "usage: maw-rs <command> [args]\ncommands:\n  auto-wake <target> --site <view|hey|api-send|api-wake|peek|bud|wake-cmd> [--fleet-known|--unknown-fleet] [--live|--not-live] [--wake] [--no-wake] [--canonical-target] [--manifest-source <source>]... [--manifest-live <true|false>] [--plan-json]
  auto-wake constants [--plan-json]
  auth sign-v1 --token <token> --now <ts> [--method <method>] [--path <path>] [--body-hash <sha256>] [--plan-json]\n  auth sign-headers --token <token> --now <ts> [--method <method>] [--path <path>] [--body <body>] [--plan-json]\n  auth verify-v1 --token <token> --signature <hex> --signed-at <ts> --now <ts> [--method <method>] [--path <path>] [--body-hash <sha256>] [--plan-json]\n  auth verify-legacy-from --from <oracle:node> --signed-at <iso> --signature <hex> --now <ts> [--cached-pubkey <key>] [--method <method>] [--path <path>] [--body <body>] [--plan-json]\n  auth verify-v3-from --from <oracle:node> --timestamp <ts> --signature-v3 <hex> --now <ts> [--cached-pubkey <key>] [--method <method>] [--path <path>] [--body <body>] [--plan-json]\n  auth from-sign-payload --from <oracle:node> (--timestamp <ts>|--legacy --signed-at <iso>) [--method <method>] [--path <path>] [--body-hash <sha256>] [--plan-json]\n  auth hmac-sign --secret <secret> --payload <payload> [--plan-json]\n  auth hmac-verify --secret <secret> --payload <payload> --signature <hex> [--plan-json]\n  auth constants [--plan-json]\n  auth sign-v3 --peer-key <hex> --from <addr> [--method <method>] [--path <path>] [--now <ts>] [--body <body>] [--plan-json]\n  auth verify-request [--method <method>] [--path <path>] [--now <ts>] [--body <body>] [--cached-pubkey <hex>] [--header <KEY=VALUE>]... [--plan-json]\n  auth loopback --address <address> [--plan-json]\n  auth from-address --node <node> [--oracle <oracle>] [--plan-json]\n  auth hash-body [--body <body>] [--plan-json]\n  hub validate-workspace --name <name> --url <url> [--plan-json]\n  hub load-workspaces --dir <dir> [--plan-json]\n  hub constants [--plan-json]\n  xdg paths [--home <dir>] [--env <KEY=VALUE>]... [--plan-json]\n  xdg core-paths [--home <dir>] [--env <KEY=VALUE>]... [--plan-json]\n  xdg validate-instance --name <name> [--plan-json]\n  xdg constants [--plan-json]\n  plugin-scaffold validate-name --name <name> [--plan-json]\n  plugin-scaffold manifest --name <name> (--rust|--as) [--plan-json]\n  plugin-scaffold constants [--plan-json]\n  policy [--constants|--weight <i32>|--default-active <key> [--includes <plugin>]] [--plan-json]\n  policy constants [--plan-json]\n  plugin-policy [--constants|--weight <i32>|--default-active <key> [--includes <plugin>]] [--plan-json]\n  plugin-manifest parse --dir <dir> --json <json> [--plan-json]\n  plugin-manifest load --dir <dir> [--plan-json]\n  plugin-manifest discover --scan-dir <dir>... [--disabled <name>]... [--runtime-version <version>] [--use-cache] [--plan-json]\n  plugin-manifest import-symbol --scan-dir <dir>... --plugin <name> --symbol <name> [--module-symbol <name=value>]... [--disabled <name>]... [--runtime-version <version>] [--plan-json]\n  plugin-manifest invoke --scan-dir <dir>... --plugin <name> [--source <cli|api|peer>] [--arg <arg>]... [--fake-ts-output <text>] [--fake-wasm-output <text>] [--disabled <name>]... [--runtime-version <version>] [--plan-json]\n  bind-host [--config-peers-len <n>] [--config-named-peers-len <n>] [--maw-host <host>] [--peers-store-len <n>|--peers-store-error <err>] [--plan-json]\n  bind-host constants [--plan-json]\n  bring|b <oracle> [--to <session[:window]>] [--plan-json]\n  ls [<peer>] [--all] [--json|--plan-json] [--compact|-c] [--verbose|-v] [--recent|-r [N]] [--active [30m|1h]] [--channels] [--pane <id|command|target|title|pid|cwd|last_activity>]...\n  feed parse-line <line> [--plan-json]\n  feed describe <event> [--message <message>] [--plan-json]\n  feed active --now <ms> --window <ms> [--event <oracle:ts:message>]... [--plan-json]\n  feed constants [--plan-json]\n  fuzzy distance <left> <right> [--plan-json]\n  fuzzy match <input> [--candidate <candidate>]... [--max-results <n>] [--max-distance <n>] [--plan-json]\n  fuzzy constants [--plan-json]\n  resolve --mode <by-name|session|worktree> <target> <item...> [--plan-json]\n  resolve constants [--plan-json]\n  identity session-name <oracle> [--slot <0-99>] [--plan-json]\n  identity node-identity <host> [--user <user>] [--plan-json]\n  identity constants [--plan-json]\n  normalize <target> [--plan-json]\n  normalize constants [--plan-json]\n  calver --now <YYYY-M-DTHH:MM> [--stable|--alpha|--beta] [--package-version <version>] [--tag <tag>]... [--plan-json]\n  calver constants [--plan-json]\n  worktree-window --main-repo-name <repo> --wt-name <worktree> [--session <name>] [--window <index:name:active>]... [--plan-json]\n  worktree-window constants [--plan-json]\n  route --query <target> [--node <name>] [--named-peer <name=url>] [--peer <url>] [--agent <agent=node>] [--session <name>] [--source <source>] [--window <index:name:active>]... [--plan-json]\n  route constants [--plan-json]\n  discover [--peers config|scout|both] [--peer <url>] [--named-peer <name=url>] [--discovered <node|host|oracle|locator[,locator]>]... [--pane <id|command|target|title|pid|cwd|last_activity>]... [--json] [--tree] [--awake] [--plan-json]
  discover constants [--plan-json]\n  federation-health [--node <name>] [--local-url <url>] [--peer <url|node|-|reachable|unreachable|latency|-|agents|ok|clock>]... [--remote <url|kind|...>]... [--plan-json]
  federation-health constants [--plan-json]\n  federation-identity [--node <name>] [--url <url>] [--agent <oracle=node>]... [--plan-json]
  federation-identity constants [--plan-json]\n  federation-sync [--node <name>] [--agent <oracle=node>]... [--identity <peer|url|node|agents|reachable|unreachable[,error]>]... [--dry-run] [--check] [--force] [--prune] [--plan-json]
  federation-sync constants [--plan-json]\n  auto-pair-proof --node <node> --oracle <oracle> --url <url> --pubkey <pubkey> --token <token> [--proof <hex>] [--plan-json]\n  consent-constants [--plan-json]\n  consent-pin (--pin <pin> [--expected-hash <sha256>]|--request-id-bytes <b0,b1,...>) [--plan-json]\n  consent-request --from <from> --to <to> --action <hey|team-invite|plugin-install> --summary <summary> --request-id <id> --pin <pin> --now <ms> [--peer-url <url>] [--peer-ok|--peer-http-status <status>|--peer-network-error <message>] [--plan-json]\n  consent-store <trust|pending> [--entry <from=...,to=...,action=...,approved_at=...,approved_by=...>]... [--request <id=...,from=...,to=...,action=...,summary=...,pin_hash=...,created_at=...,expires_at=...,status=...>]... [--check <from:to:action>] [--key <from:to:action>] [--set-status <id:status>] [--plan-json]\n  consent-expiry --request <id=...,from=...,to=...,action=...,summary=...,pin_hash=...,created_at=...,expires_at=...,status=...> --now <ms> [--plan-json]\n  consent-cleanup --request <id=...,from=...,to=...,action=...,summary=...,pin_hash=...,created_at=...,expires_at=...,status=...>... --delete <id> [--plan-json]\n  consent-trust-revoke [--entry <from=...,to=...,action=...,approved_at=...,approved_by=...>]... --revoke <from:to:action> [--plan-json]\n  consent-trust-check [--entry <from=...,to=...,action=...,approved_at=...,approved_by=...>]... --check <from:to:action> [--plan-json]\n  consent-pending-read [--request <id=...,from=...,to=...,action=...,summary=...,pin_hash=...,created_at=...,expires_at=...,status=...>]... --id <id> [--plan-json]\n  consent-pending-status [--request <id=...,from=...,to=...,action=...,summary=...,pin_hash=...,created_at=...,expires_at=...,status=...>]... --set-status <id:pending|approved|rejected|expired> [--plan-json]\n  recent-hello [--hello <zid:seen_at_ms>]... --zid <zid> --now <ms> [--plan-json]\n  recent-hello constants [--plan-json]\n  pair-code (--code <code>|--bytes <b0,b1,...>) [--plan-json]\n  pair-code constants [--plan-json]\n  pair-code-store <register|lookup|consume> --code <code> --now <ms> [--ttl-ms <ms>] [--seed-code <code:ttl_ms:created_at_ms>]... [--plan-json]\n  pair-code-store constants [--plan-json]\n  pair-api <generate|probe|accept|status> --code <code> --node <node> --oracle <oracle> --port <port> --base-url <url> --federation-token <token> --pubkey <pubkey> --now <ms> [--plan-json]\n  pair-api constants [--plan-json]\n  pair-api-auto --node <node> --oracle <oracle> --port <port> --base-url <url> --federation-token <token> --pubkey <pubkey> --now <ms> [--plan-json]\n  pair-api-auto constants [--plan-json]\n  peer-probe classify (--http-status <n>|--code <code>|--cause-code <code>|--name <name>|--non-object) [--plan-json]
  peer-probe constants [--plan-json]
  peer-probe format --code <code> --message <msg> --url <url> --alias <alias> [--at <ts>] [--plan-json]
  peer-probe handshake (--legacy-true|--schema <schema>|--empty-object|--other-truthy|--missing) [--plan-json]
  peer-sources --mode <config|scout|both> [--peer <url>] [--named-peer <name=url>] [--discovery-ok|--discovery-error <error>] [--discovery-hint <hint>] [--discovered <node|host|oracle|locator[,locator]>]... [--plan-json]
  peer-sources constants [--plan-json]\n  policy [--constants|--weight <i32>|--default-active <key> [--includes <plugin>]] [--plan-json]\n  split-policy [--pane-current-command <cmd>] [--requested-policy <policy>] [--no-attach] [--force-split] [--plan-json]\n  split-policy constants [--plan-json]\n  transport --classify-error <error>|--classify-empty|--send [--transport <name[:connected][:canReach][:ok|false|throw=err]>]... [--plan-json]\n  transport constants [--plan-json]\n"
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LsPanePlan {
    id: String,
    target: String,
    session: String,
    command: String,
    title: String,
    source: Option<String>,
    last_activity: Option<u64>,
    session_created: Option<u64>,
    status: &'static str,
    age_sec: u64,
    agent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LsMode {
    Compact,
    Verbose,
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct LsPlanOptions {
    json: bool,
    mode: LsMode,
    all: bool,
    channels: bool,
    active: bool,
    active_threshold_sec: Option<u64>,
    recent: bool,
    recent_limit: Option<usize>,
    filter: Option<String>,
    peer: Option<String>,
    now: Option<u64>,
    panes: Vec<TmuxPane>,
    session_created: BTreeMap<String, u64>,
}

fn run_ls_plan(argv: &[String]) -> CliOutput {
    match parse_ls_plan_options(argv) {
        Ok(options) => render_ls_plan(&options),
        Err(output) => output,
    }
}

#[allow(clippy::too_many_lines)]
fn parse_ls_plan_options(argv: &[String]) -> Result<LsPlanOptions, CliOutput> {
    let mut options = LsPlanOptions {
        json: false,
        mode: LsMode::Compact,
        all: false,
        channels: false,
        active: false,
        active_threshold_sec: None,
        recent: false,
        recent_limit: None,
        filter: None,
        peer: None,
        now: None,
        panes: Vec::new(),
        session_created: BTreeMap::new(),
    };

    let mut positionals = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--help" | "-h" => return Err(ls_help_ok()),
            "--json" | "--plan-json" => options.json = true,
            "--all" => options.all = true,
            "--compact" | "-c" => options.mode = LsMode::Compact,
            "--verbose" | "-v" => options.mode = LsMode::Verbose,
            "--channels" => options.channels = true,
            "--active" => {
                options.active = true;
                if let Some(next) = argv.get(index + 1) {
                    if !next.starts_with('-') {
                        if let Some(seconds) = parse_ls_duration_seconds(next) {
                            options.active_threshold_sec = Some(seconds);
                            index += 1;
                        }
                    }
                }
            }
            arg if arg.starts_with("--active=") => {
                options.active = true;
                let raw = &arg["--active=".len()..];
                let Some(seconds) = parse_ls_duration_seconds(raw) else {
                    return Err(ls_usage_error("ls: invalid --active duration"));
                };
                options.active_threshold_sec = Some(seconds);
            }
            "--recent" | "-r" => {
                options.recent = true;
                options.all = true;
                if let Some(next) = argv.get(index + 1) {
                    if !next.starts_with('-') {
                        if let Ok(limit) = next.parse::<usize>() {
                            options.recent_limit = Some(limit);
                            index += 1;
                        }
                    }
                }
            }
            "--pane" => {
                let Some(value) = argv.get(index + 1) else {
                    return Err(ls_usage_error("ls: missing --pane value"));
                };
                match parse_ls_pane(value) {
                    Ok(pane) => options.panes.push(pane),
                    Err(message) => return Err(ls_usage_error(&message)),
                }
                index += 1;
            }
            "--now" => {
                let Some(value) = argv.get(index + 1) else {
                    return Err(ls_usage_error("ls: missing --now value"));
                };
                match value.parse::<u64>() {
                    Ok(value) => options.now = Some(value),
                    Err(_) => return Err(ls_usage_error("ls: --now must be an integer")),
                }
                index += 1;
            }
            "--session-created" => {
                let Some(value) = argv.get(index + 1) else {
                    return Err(ls_usage_error("ls: missing --session-created value"));
                };
                let Some((session, created)) = value.split_once('=') else {
                    return Err(ls_usage_error(
                        "ls: --session-created must use <session=epoch_seconds>",
                    ));
                };
                match created.parse::<u64>() {
                    Ok(created) => {
                        options.session_created.insert(session.to_owned(), created);
                    }
                    Err(_) => {
                        return Err(ls_usage_error(
                            "ls: session-created epoch must be an integer",
                        ));
                    }
                }
                index += 1;
            }
            "--verify" | "--fix" => {
                // Accepted for top-level maw-js surface parity. This plan-only
                // port does not mutate/prune worktrees.
            }
            "-a" => {
                options.all = true;
            }
            arg if arg.starts_with('-') => {
                return Err(ls_usage_error(&format!("ls: unknown argument {arg}")));
            }
            arg => positionals.push(arg.to_owned()),
        }
        index += 1;
    }

    if let Some(first) = positionals.first() {
        if options.active || !options.panes.is_empty() {
            options.filter = Some(first.clone());
        } else {
            options.peer = Some(first.clone());
        }
    }

    Ok(options)
}

