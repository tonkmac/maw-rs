//! Minimal side-by-side maw-rs CLI dry-run surfaces.
//!
//! This crate intentionally starts with plan-only output so command parity can
//! be tested against maw-js parser contracts before host IO is wired.

use maw_bring::{parse_bring_args, BringAliasOptions, ParsedBringArgs};
use maw_calver::{compute_version, Channel, ComputeArgs, DateParts};
use maw_matcher::{
    normalize_target, resolve_by_name, resolve_session_target, resolve_worktree_target,
    ResolveOptions, ResolveResult,
};
use maw_peer::{
    resolve_peer_sources, DiscoveryResult, DiscoveryRow, NamedPeerConfig, PeerConfig,
    PeerSourceMode, PeerSourceResult,
};
use maw_routing::{
    resolve_target as resolve_route_target, MawConfig as RouteConfig, NamedPeer as RouteNamedPeer,
    ResolveResult as RouteResult, Session as RouteSession, Window as RouteWindow,
};
use maw_worktree::{
    resolve_worktree_window, Session as WorktreeSession, Window as WorktreeWindow,
    WorktreeWindowResolution,
};
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run the current maw-rs CLI parser/renderer over argv without process exit.
#[must_use]
pub fn run_cli(argv: &[String]) -> CliOutput {
    let Some(command) = argv.first().map(String::as_str) else {
        return usage_ok();
    };
    match command {
        "--help" | "-h" | "help" => usage_ok(),
        "bring" | "b" => run_bring_plan(&argv[1..]),
        "resolve" => run_resolve_plan(&argv[1..]),
        "normalize" => run_normalize_plan(&argv[1..]),
        "calver" => run_calver_plan(&argv[1..]),
        "worktree-window" => run_worktree_window_plan(&argv[1..]),
        "route" => run_route_plan(&argv[1..]),
        "peer-sources" => run_peer_sources_plan(&argv[1..]),
        _ => CliOutput {
            code: 2,
            stdout: String::new(),
            stderr: format!("unknown command: {command}\n{}", usage_text()),
        },
    }
}

fn run_peer_sources_plan(argv: &[String]) -> CliOutput {
    let mut plan_json = false;
    let mut mode = PeerSourceMode::Both;
    let mut config = PeerConfig::default();
    let mut discoveries: Option<DiscoveryResult> = None;
    let mut discovery_rows = Vec::new();
    let mut discovery_error_hint = None;

    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--plan-json" => plan_json = true,
            "--mode" => {
                let Some(value) = argv.get(index + 1) else {
                    return peer_sources_usage_error("peer-sources: missing --mode value");
                };
                let Some(parsed) = maw_peer::parse_peer_source_mode(Some(value), mode) else {
                    return peer_sources_usage_error("peer-sources: unknown --mode");
                };
                mode = parsed;
                index += 1;
            }
            "--peer" => {
                let Some(value) = argv.get(index + 1) else {
                    return peer_sources_usage_error("peer-sources: missing --peer value");
                };
                config.peers.push(value.to_owned());
                index += 1;
            }
            "--named-peer" => {
                let Some(value) = argv.get(index + 1) else {
                    return peer_sources_usage_error("peer-sources: missing --named-peer value");
                };
                match parse_key_value(value, "peer-sources: --named-peer must use <name=url>") {
                    Ok((name, url)) => config.named_peers.push(NamedPeerConfig { name, url }),
                    Err(message) => return peer_sources_usage_error(&message),
                }
                index += 1;
            }
            "--discovery-ok" => discoveries = Some(DiscoveryResult::Ok { peers: Vec::new() }),
            "--discovery-error" => {
                let Some(value) = argv.get(index + 1) else {
                    return peer_sources_usage_error(
                        "peer-sources: missing --discovery-error value",
                    );
                };
                discoveries = Some(DiscoveryResult::Err {
                    error: value.to_owned(),
                    hint: discovery_error_hint.clone(),
                });
                index += 1;
            }
            "--discovery-hint" => {
                let Some(value) = argv.get(index + 1) else {
                    return peer_sources_usage_error(
                        "peer-sources: missing --discovery-hint value",
                    );
                };
                discovery_error_hint = Some(value.to_owned());
                if let Some(DiscoveryResult::Err { hint, .. }) = &mut discoveries {
                    hint.clone_from(&discovery_error_hint);
                }
                index += 1;
            }
            "--discovered" => {
                let Some(value) = argv.get(index + 1) else {
                    return peer_sources_usage_error("peer-sources: missing --discovered value");
                };
                match parse_discovery_row(value) {
                    Ok(row) => discovery_rows.push(row),
                    Err(message) => return peer_sources_usage_error(&message),
                }
                index += 1;
            }
            arg => {
                return peer_sources_usage_error(&format!("peer-sources: unknown argument {arg}"))
            }
        }
        index += 1;
    }

    if !discovery_rows.is_empty() {
        discoveries = Some(DiscoveryResult::Ok {
            peers: discovery_rows,
        });
    }

    let result = resolve_peer_sources(&config, mode, discoveries.as_ref());
    CliOutput {
        code: 0,
        stdout: if plan_json {
            render_peer_sources_plan_json(&result)
        } else {
            render_peer_sources_plan_text(&result)
        },
        stderr: String::new(),
    }
}

fn peer_sources_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{message}\nusage: maw-rs peer-sources --mode <config|scout|both> [--peer <url>] [--named-peer <name=url>] [--discovery-ok|--discovery-error <error>] [--discovery-hint <hint>] [--discovered <node|host|oracle|locator[,locator]>]... [--plan-json]\n"
        ),
    }
}

fn parse_discovery_row(value: &str) -> Result<DiscoveryRow, String> {
    let parts: Vec<&str> = value.splitn(4, '|').collect();
    if parts.len() != 4 {
        return Err(
            "peer-sources: --discovered must use <node|host|oracle|locator[,locator]>".to_owned(),
        );
    }
    Ok(DiscoveryRow {
        node: optional_field(parts[0]),
        host: optional_field(parts[1]),
        oracle: optional_field(parts[2]),
        locators: parts[3]
            .split(',')
            .filter(|locator| !locator.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    })
}

fn optional_field(value: &str) -> Option<String> {
    if value.is_empty() || value == "-" {
        None
    } else {
        Some(value.to_owned())
    }
}

fn render_peer_sources_plan_json(result: &PeerSourceResult) -> String {
    format!(
        "{{\"command\":\"peer-sources\",\"mode\":{},\"peers\":{},\"warnings\":{},\"fetchCalls\":{}}}\n",
        json_string(result.mode.as_str()),
        render_peer_targets_json(result),
        json_string_array(&result.warnings),
        result.fetch_calls
    )
}

fn render_peer_targets_json(result: &PeerSourceResult) -> String {
    format!(
        "[{}]",
        result
            .peers
            .iter()
            .map(|peer| {
                let mut fields = vec![
                    format!("\"url\":{}", json_string(&peer.url)),
                    format!("\"source\":{}", json_string(peer.source.as_str())),
                ];
                push_json_opt(&mut fields, "name", peer.name.as_deref());
                push_json_opt(&mut fields, "node", peer.node.as_deref());
                push_json_opt(&mut fields, "oracle", peer.oracle.as_deref());
                format!("{{{}}}", fields.join(","))
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_peer_sources_plan_text(result: &PeerSourceResult) -> String {
    let mut lines = vec![format!(
        "peer-sources mode={} fetchCalls={}",
        result.mode.as_str(),
        result.fetch_calls
    )];
    for peer in &result.peers {
        lines.push(format!(
            "{} {} {}",
            peer.source.as_str(),
            peer.name.as_deref().unwrap_or("-"),
            peer.url
        ));
    }
    for warning in &result.warnings {
        lines.push(format!("warning: {warning}"));
    }
    lines.join("\n") + "\n"
}

#[allow(clippy::too_many_lines)]
fn run_route_plan(argv: &[String]) -> CliOutput {
    let mut plan_json = false;
    let mut query = None;
    let mut config = RouteConfig::default();
    let mut sessions: Vec<RouteSession> = Vec::new();
    let mut current_session: Option<RouteSession> = None;

    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--plan-json" => plan_json = true,
            "--query" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --query value");
                };
                query = Some(value.to_owned());
                index += 1;
            }
            "--node" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --node value");
                };
                config.node = Some(value.to_owned());
                index += 1;
            }
            "--named-peer" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --named-peer value");
                };
                match parse_key_value(value, "route: --named-peer must use <name=url>") {
                    Ok((name, url)) => config.named_peers.push(RouteNamedPeer { name, url }),
                    Err(message) => return route_usage_error(&message),
                }
                index += 1;
            }
            "--peer" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --peer value");
                };
                config.peers.push(value.to_owned());
                index += 1;
            }
            "--agent" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --agent value");
                };
                match parse_key_value(value, "route: --agent must use <agent=node>") {
                    Ok((agent, node)) => {
                        config.agents.insert(agent, node);
                    }
                    Err(message) => return route_usage_error(&message),
                }
                index += 1;
            }
            "--session" => {
                if let Some(session) = current_session.take() {
                    sessions.push(session);
                }
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --session value");
                };
                current_session = Some(RouteSession {
                    name: value.to_owned(),
                    windows: Vec::new(),
                    source: None,
                });
                index += 1;
            }
            "--source" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --source value");
                };
                let Some(session) = &mut current_session else {
                    return route_usage_error("route: --source must follow a --session");
                };
                session.source = Some(value.to_owned());
                index += 1;
            }
            "--window" => {
                let Some(value) = argv.get(index + 1) else {
                    return route_usage_error("route: missing --window value");
                };
                let Some(session) = &mut current_session else {
                    return route_usage_error("route: --window must follow a --session");
                };
                match parse_route_window(value) {
                    Ok(window) => session.windows.push(window),
                    Err(message) => return route_usage_error(&message),
                }
                index += 1;
            }
            arg => return route_usage_error(&format!("route: unknown argument {arg}")),
        }
        index += 1;
    }
    if let Some(session) = current_session.take() {
        sessions.push(session);
    }

    let Some(query) = query else {
        return route_usage_error("route: expected --query <target>");
    };
    let result = resolve_route_target(&query, &config, &sessions);
    CliOutput {
        code: 0,
        stdout: if plan_json {
            render_route_plan_json(&query, &result)
        } else {
            render_route_plan_text(&query, &result)
        },
        stderr: String::new(),
    }
}

fn route_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{message}\nusage: maw-rs route --query <target> [--node <name>] [--named-peer <name=url>] [--peer <url>] [--agent <agent=node>] [--session <name>] [--source <source>] [--window <index:name:active>]... [--plan-json]\n"
        ),
    }
}

fn parse_key_value(value: &str, message: &str) -> Result<(String, String), String> {
    let Some((key, value)) = value.split_once('=') else {
        return Err(message.to_owned());
    };
    if key.is_empty() || value.is_empty() {
        return Err(message.to_owned());
    }
    Ok((key.to_owned(), value.to_owned()))
}

fn parse_route_window(value: &str) -> Result<RouteWindow, String> {
    let mut parts = value.splitn(3, ':');
    let index = parts
        .next()
        .ok_or_else(|| "route: missing window index".to_owned())?
        .parse::<u32>()
        .map_err(|_| "route: invalid window index".to_owned())?;
    let Some(name) = parts.next() else {
        return Err("route: window must use <index:name:active>".to_owned());
    };
    let active = match parts.next() {
        Some("true") => true,
        Some("false") => false,
        _ => return Err("route: window active must be true or false".to_owned()),
    };
    Ok(RouteWindow {
        index,
        name: name.to_owned(),
        active,
    })
}

fn render_route_plan_json(query: &str, result: &RouteResult) -> String {
    let mut fields = vec![
        "\"command\":\"route\"".to_owned(),
        format!("\"query\":{}", json_string(query)),
    ];
    match result {
        RouteResult::Local { target } => {
            fields.push("\"type\":\"local\"".to_owned());
            fields.push(format!("\"target\":{}", json_string(target)));
        }
        RouteResult::Peer {
            peer_url,
            target,
            node,
        } => {
            fields.push("\"type\":\"peer\"".to_owned());
            fields.push(format!("\"peerUrl\":{}", json_string(peer_url)));
            fields.push(format!("\"target\":{}", json_string(target)));
            fields.push(format!("\"node\":{}", json_string(node)));
        }
        RouteResult::SelfNode { target } => {
            fields.push("\"type\":\"self-node\"".to_owned());
            fields.push(format!("\"target\":{}", json_string(target)));
        }
        RouteResult::Error {
            reason,
            detail,
            hint,
        } => {
            fields.push("\"type\":\"error\"".to_owned());
            fields.push(format!("\"reason\":{}", json_string(reason)));
            fields.push(format!("\"detail\":{}", json_string(detail)));
            if let Some(hint) = hint {
                fields.push(format!("\"hint\":{}", json_string(hint)));
            }
        }
    }
    format!("{{{}}}\n", fields.join(","))
}

fn render_route_plan_text(query: &str, result: &RouteResult) -> String {
    match result {
        RouteResult::Local { target } => format!("route {query}: local {target}\n"),
        RouteResult::Peer {
            peer_url,
            target,
            node,
        } => format!("route {query}: peer {node} {target} via {peer_url}\n"),
        RouteResult::SelfNode { target } => format!("route {query}: self-node {target}\n"),
        RouteResult::Error {
            reason,
            detail,
            hint,
        } => hint.as_ref().map_or_else(
            || format!("route {query}: error {reason} {detail}\n"),
            |hint| format!("route {query}: error {reason} {detail} hint={hint}\n"),
        ),
    }
}

fn run_worktree_window_plan(argv: &[String]) -> CliOutput {
    let mut plan_json = false;
    let mut main_repo_name = None;
    let mut wt_name = None;
    let mut sessions: Vec<WorktreeSession> = Vec::new();
    let mut current_session: Option<WorktreeSession> = None;

    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--plan-json" => plan_json = true,
            "--main-repo-name" => {
                let Some(value) = argv.get(index + 1) else {
                    return worktree_window_usage_error(
                        "worktree-window: missing --main-repo-name value",
                    );
                };
                main_repo_name = Some(value.to_owned());
                index += 1;
            }
            "--wt-name" => {
                let Some(value) = argv.get(index + 1) else {
                    return worktree_window_usage_error("worktree-window: missing --wt-name value");
                };
                wt_name = Some(value.to_owned());
                index += 1;
            }
            "--session" => {
                if let Some(session) = current_session.take() {
                    sessions.push(session);
                }
                let Some(value) = argv.get(index + 1) else {
                    return worktree_window_usage_error("worktree-window: missing --session value");
                };
                current_session = Some(WorktreeSession {
                    name: value.to_owned(),
                    windows: Vec::new(),
                });
                index += 1;
            }
            "--window" => {
                let Some(value) = argv.get(index + 1) else {
                    return worktree_window_usage_error("worktree-window: missing --window value");
                };
                let Some(session) = &mut current_session else {
                    return worktree_window_usage_error(
                        "worktree-window: --window must follow a --session",
                    );
                };
                match parse_worktree_window(value) {
                    Ok(window) => session.windows.push(window),
                    Err(message) => return worktree_window_usage_error(&message),
                }
                index += 1;
            }
            arg => {
                return worktree_window_usage_error(&format!(
                    "worktree-window: unknown argument {arg}"
                ));
            }
        }
        index += 1;
    }
    if let Some(session) = current_session.take() {
        sessions.push(session);
    }

    let Some(main_repo_name) = main_repo_name else {
        return worktree_window_usage_error("worktree-window: expected --main-repo-name <repo>");
    };
    let Some(wt_name) = wt_name else {
        return worktree_window_usage_error("worktree-window: expected --wt-name <worktree>");
    };

    let result = resolve_worktree_window(&main_repo_name, &wt_name, &sessions);
    CliOutput {
        code: 0,
        stdout: if plan_json {
            render_worktree_window_plan_json(&main_repo_name, &wt_name, &result)
        } else {
            render_worktree_window_plan_text(&main_repo_name, &wt_name, &result)
        },
        stderr: String::new(),
    }
}

fn worktree_window_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{message}\nusage: maw-rs worktree-window --main-repo-name <repo> --wt-name <worktree> [--session <name>] [--window <index:name:active>]... [--plan-json]\n"
        ),
    }
}

fn parse_worktree_window(value: &str) -> Result<WorktreeWindow, String> {
    let mut parts = value.splitn(3, ':');
    let index = parts
        .next()
        .ok_or_else(|| "worktree-window: missing window index".to_owned())?
        .parse::<u32>()
        .map_err(|_| "worktree-window: invalid window index".to_owned())?;
    let Some(name) = parts.next() else {
        return Err("worktree-window: window must use <index:name:active>".to_owned());
    };
    let active = match parts.next() {
        Some("true") => true,
        Some("false") => false,
        _ => return Err("worktree-window: window active must be true or false".to_owned()),
    };
    Ok(WorktreeWindow {
        index,
        name: name.to_owned(),
        active,
    })
}

fn render_worktree_window_plan_json(
    main_repo_name: &str,
    wt_name: &str,
    result: &WorktreeWindowResolution,
) -> String {
    let mut fields = vec![
        "\"command\":\"worktree-window\"".to_owned(),
        format!("\"mainRepoName\":{}", json_string(main_repo_name)),
        format!("\"wtName\":{}", json_string(wt_name)),
    ];
    match result {
        WorktreeWindowResolution::Bound { window } => {
            fields.push("\"kind\":\"bound\"".to_owned());
            fields.push(format!("\"window\":{}", json_string(window)));
        }
        WorktreeWindowResolution::Ambiguous { query, candidates } => {
            fields.push("\"kind\":\"ambiguous\"".to_owned());
            fields.push(format!("\"query\":{}", json_string(query)));
            fields.push(format!("\"candidates\":{}", json_string_array(candidates)));
        }
        WorktreeWindowResolution::None => fields.push("\"kind\":\"none\"".to_owned()),
    }
    format!("{{{}}}\n", fields.join(","))
}

fn render_worktree_window_plan_text(
    main_repo_name: &str,
    wt_name: &str,
    result: &WorktreeWindowResolution,
) -> String {
    match result {
        WorktreeWindowResolution::Bound { window } => {
            format!("worktree-window {main_repo_name} {wt_name}: bound {window}\n")
        }
        WorktreeWindowResolution::Ambiguous { query, candidates } => format!(
            "worktree-window {main_repo_name} {wt_name}: ambiguous {query} candidates={}\n",
            candidates.join(", ")
        ),
        WorktreeWindowResolution::None => {
            format!("worktree-window {main_repo_name} {wt_name}: none\n")
        }
    }
}

fn run_calver_plan(argv: &[String]) -> CliOutput {
    let mut plan_json = false;
    let mut stable = false;
    let mut channel = None;
    let mut now = None;
    let mut package_version = String::new();
    let mut tags = Vec::new();

    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--plan-json" => plan_json = true,
            "--stable" => stable = true,
            "--alpha" => channel = Some(Channel::Alpha),
            "--beta" => channel = Some(Channel::Beta),
            "--now" => {
                let Some(value) = argv.get(index + 1) else {
                    return calver_usage_error("calver: missing --now value");
                };
                match parse_date_parts(value) {
                    Ok(parsed) => now = Some(parsed),
                    Err(message) => return calver_usage_error(&message),
                }
                index += 1;
            }
            "--package-version" => {
                let Some(value) = argv.get(index + 1) else {
                    return calver_usage_error("calver: missing --package-version value");
                };
                package_version.clone_from(value);
                index += 1;
            }
            "--tag" => {
                let Some(value) = argv.get(index + 1) else {
                    return calver_usage_error("calver: missing --tag value");
                };
                tags.push(value.to_owned());
                index += 1;
            }
            arg => return calver_usage_error(&format!("calver: unknown argument {arg}")),
        }
        index += 1;
    }

    let Some(now) = now else {
        return calver_usage_error("calver: expected --now <YYYY-M-DTHH:MM>");
    };

    let compute_args = ComputeArgs {
        stable,
        channel,
        now,
    };
    match compute_version(compute_args, &tags, &package_version) {
        Ok(version) => CliOutput {
            code: 0,
            stdout: if plan_json {
                render_calver_plan_json(compute_args, &tags, &package_version, &version)
            } else {
                format!("{version}\n")
            },
            stderr: String::new(),
        },
        Err(error) => CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("calver: {error}\n"),
        },
    }
}

fn calver_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{message}\nusage: maw-rs calver --now <YYYY-M-DTHH:MM> [--stable|--alpha|--beta] [--package-version <version>] [--tag <tag>]... [--plan-json]\n"
        ),
    }
}

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
    let plan_json = argv.iter().any(|arg| arg == "--plan-json");
    let target = argv.iter().find(|arg| arg.as_str() != "--plan-json");
    let Some(target) = target else {
        return CliOutput {
            code: 2,
            stdout: String::new(),
            stderr:
                "normalize: expected <target>\nusage: maw-rs normalize <target> [--plan-json]\n"
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

fn run_resolve_plan(argv: &[String]) -> CliOutput {
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
        stderr: format!("{message}\nusage: maw-rs resolve --mode <by-name|session|worktree> <target> <item...> [--plan-json]\n"),
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

fn usage_ok() -> CliOutput {
    CliOutput {
        code: 0,
        stdout: usage_text(),
        stderr: String::new(),
    }
}

fn usage_text() -> String {
    "usage: maw-rs <command> [args]\ncommands:\n  bring|b <oracle> [--to <session[:window]>] [--plan-json]\n  resolve --mode <by-name|session|worktree> <target> <item...> [--plan-json]\n  normalize <target> [--plan-json]\n  calver --now <YYYY-M-DTHH:MM> [--stable|--alpha|--beta] [--package-version <version>] [--tag <tag>]... [--plan-json]\n  worktree-window --main-repo-name <repo> --wt-name <worktree> [--session <name>] [--window <index:name:active>]... [--plan-json]\n  route --query <target> [--node <name>] [--named-peer <name=url>] [--peer <url>] [--agent <agent=node>] [--session <name>] [--source <source>] [--window <index:name:active>]... [--plan-json]\n  peer-sources --mode <config|scout|both> [--peer <url>] [--named-peer <name=url>] [--discovery-ok|--discovery-error <error>] [--discovery-hint <hint>] [--discovered <node|host|oracle|locator[,locator]>]... [--plan-json]\n"
        .to_owned()
}

fn run_bring_plan(argv: &[String]) -> CliOutput {
    let plan_json = argv.iter().any(|arg| arg == "--plan-json");
    let filtered: Vec<String> = argv
        .iter()
        .filter(|arg| arg.as_str() != "--plan-json")
        .cloned()
        .collect();
    match parse_bring_args(&filtered) {
        Ok(parsed) => CliOutput {
            code: 0,
            stdout: if plan_json {
                render_bring_plan_json(&parsed)
            } else {
                render_bring_plan_text(&parsed)
            },
            stderr: String::new(),
        },
        Err(error) => CliOutput {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}\n{}\n", error.message, error.usage.join("\n")),
        },
    }
}

fn render_bring_plan_text(parsed: &ParsedBringArgs) -> String {
    let mut lines = vec![format!("wake {} --split", parsed.oracle)];
    if let Some(engine) = &parsed.opts.engine {
        lines.push(format!("engine: {engine}"));
    }
    if let Some(session) = &parsed.opts.session {
        lines.push(format!("session: {session}"));
    }
    if let Some(split_target) = &parsed.opts.split_target {
        lines.push(format!("split-target: {split_target}"));
    }
    if parsed.opts.pick {
        lines.push("pick: true".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_bring_plan_json(parsed: &ParsedBringArgs) -> String {
    let opts = &parsed.opts;
    let mut fields = vec![
        format!("\"oracle\":{}", json_string(&parsed.oracle)),
        format!("\"split\":{}", opts.split),
    ];
    push_json_opt(&mut fields, "engine", opts.engine.as_deref());
    if opts.pick {
        fields.push("\"pick\":true".to_owned());
    }
    push_json_opt(&mut fields, "session", opts.session.as_deref());
    push_json_opt(&mut fields, "splitTarget", opts.split_target.as_deref());
    format!(
        "{{\"command\":\"bring\",\"opts\":{{{}}}}}\n",
        fields.join(",")
    )
}

fn push_json_opt(fields: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        fields.push(format!("{}:{}", json_string(key), json_string(value)));
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[allow(dead_code)]
const fn _assert_options_shape(_: &BringAliasOptions) {}
