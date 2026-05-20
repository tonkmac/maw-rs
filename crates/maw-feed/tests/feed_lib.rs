use maw_feed::{active_oracles_at, describe_activity, parse_line, FeedEvent};

fn event(overrides: impl FnOnce(&mut FeedEvent)) -> FeedEvent {
    let mut event = FeedEvent {
        timestamp: "2026-05-18 12:00:00".to_owned(),
        oracle: "oracle-a".to_owned(),
        host: "m5".to_owned(),
        event: "Notification".to_owned(),
        project: "maw-js".to_owned(),
        session_id: "s1".to_owned(),
        message: "hello".to_owned(),
        ts: 1_000_000,
    };
    overrides(&mut event);
    event
}

#[test]
fn parse_line_rejects_malformed_and_invalid_timestamp_lines() {
    assert_eq!(parse_line(""), None);
    assert_eq!(parse_line("not a feed row"), None);
    assert_eq!(
        parse_line("2026-05-18 12:00:00 | oracle | host | Event"),
        None
    );
    assert_eq!(
        parse_line("not-a-date | oracle | host | Notification | project | session » message"),
        None
    );
}

#[test]
fn parse_line_supports_message_no_message_fallback_and_pipe_characters_in_tail() {
    let parsed = parse_line(
        "2026-05-18 12:34:56 | alpha | m5 | PreToolUse | /repo | sess-1 » Bash: echo a | b",
    )
    .expect("valid feed line should parse");
    assert_eq!(parsed.timestamp, "2026-05-18 12:34:56");
    assert_eq!(parsed.oracle, "alpha");
    assert_eq!(parsed.host, "m5");
    assert_eq!(parsed.event, "PreToolUse");
    assert_eq!(parsed.project, "/repo");
    assert_eq!(parsed.session_id, "sess-1");
    assert_eq!(parsed.message, "Bash: echo a | b");
    assert!(parsed.ts > 0);

    let fallback = parse_line("2026-05-18 12:34:56 | beta | white | Stop | /repo | sess-only")
        .expect("fallback feed line should parse");
    assert_eq!(fallback.session_id, "sess-only");
    assert_eq!(fallback.message, "");
}

#[test]
fn active_oracles_keeps_only_recent_latest_events_per_oracle() {
    let now = 10_000;
    let stale = event(|e| {
        e.oracle = "old".to_owned();
        e.ts = now - 10_000;
    });
    let first = event(|e| {
        e.oracle = "alpha".to_owned();
        e.message = "older".to_owned();
        e.ts = now - 800;
    });
    let latest = event(|e| {
        e.oracle = "alpha".to_owned();
        e.message = "latest".to_owned();
        e.ts = now - 100;
    });
    let beta = event(|e| {
        e.oracle = "beta".to_owned();
        e.message = "beta".to_owned();
        e.ts = now - 500;
    });

    let active = active_oracles_at(&[stale, first, beta, latest], now, 1_000);
    assert_eq!(
        active.keys().cloned().collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
    assert_eq!(
        active.get("alpha").map(|e| e.message.as_str()),
        Some("latest")
    );
    assert_eq!(active.get("beta").map(|e| e.message.as_str()), Some("beta"));
}

#[test]
fn describe_activity_renders_tool_and_prompt_branches() {
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PreToolUse".to_owned();
            e.message = "Bash: run a command".to_owned();
        })),
        "⚡ Bash: run a command"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PreToolUse".to_owned();
            e.message = format!("Unknown: {}", "x".repeat(65));
        })),
        format!("🔧 Unknown: {}...", "x".repeat(57))
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PreToolUse".to_owned();
            e.message = "Read ✓".to_owned();
        })),
        "📖 Read"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PostToolUse".to_owned();
            e.message = "Bash ✓ 0".to_owned();
        })),
        "✓ Bash done"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PostToolUseFailure".to_owned();
            e.message = "Edit ✗ failed".to_owned();
        })),
        "✗ Edit failed"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "UserPromptSubmit".to_owned();
            e.message = "u".repeat(65);
        })),
        format!("💬 {}...", "u".repeat(57))
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "UserPromptSubmit".to_owned();
            e.message.clear();
        })),
        "💬 New prompt"
    );
}

#[test]
fn describe_activity_renders_lifecycle_notification_and_fallback_branches() {
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "SubagentStart".to_owned();
            e.message.clear();
        })),
        "🤖 Subagent started"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "SubagentStop".to_owned();
            e.message.clear();
        })),
        "🤖 Subagent done"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "SessionStart".to_owned();
            e.message.clear();
        })),
        "🟢 Session started"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "SessionEnd".to_owned();
            e.message.clear();
        })),
        "⏹ Session ended"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "Stop".to_owned();
            e.message.clear();
        })),
        "⏹ Stopped"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "Stop".to_owned();
            e.message = "s".repeat(65);
        })),
        format!("⏹ {}...", "s".repeat(57))
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "Notification".to_owned();
            e.message = "ping".to_owned();
        })),
        "🔔 ping"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "Notification".to_owned();
            e.message.clear();
        })),
        "🔔 Notification"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PluginError".to_owned();
            e.message = "plugin blew up".to_owned();
        })),
        "plugin blew up"
    );
    assert_eq!(
        describe_activity(&event(|e| {
            e.event = "PluginLoad".to_owned();
            e.message.clear();
        })),
        "PluginLoad"
    );
}
