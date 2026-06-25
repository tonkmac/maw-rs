const DISPATCH_98: &[DispatcherEntry] = &[
    DispatcherEntry {
        command: "trust",
        handler: Handler::Sync(trust_run_command),
    },
    DispatcherEntry {
        command: "trusts",
        handler: Handler::Sync(trust_run_command),
    },
];

const TRUST_USAGE: &str = "usage: maw trust <list|add|remove> [...]\n  list                            — list all trust entries (sorted by addedAt)\n  add      <sender> <target>      — plan adding a trust relationship (no store write)\n  remove   <sender> <target> [--yes]\n                                  — plan removing a trust relationship (confirms unless --yes)\n\nstorage: plan-only; native trust-store writes are stubbed pending design";
const TRUST_FAKE_STORE_ENV: &str = "MAW_RS_TRUST_FAKE_STORE";
const TRUST_STUB_WARNING: &str = "warn: trust native store mutation is plan-only pending native trust-store design; TODO(#115)\n";
const TRUST_AUTO_REFUSE: &str =
    "trust: consent mutation requires explicit human flow; no auto-trust surface is exposed";
const TRUST_AUTO_BLOCKLIST: &[&str] = &[
    "auto-trust",
    "auto-approve",
    "trust-all",
    "trust-everyone",
    "trust-all-peers",
    "approve-all",
    "auto",
    "auto-pair",
    "pair-auto",
    "pair-approve",
    "allow-all",
    "allowlist-all",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustEntryPlan {
    sender: String,
    target: String,
    added_at: String,
}

fn trust_run_command(argv: &[String]) -> CliOutput {
    match trust_validate_auto_surface(argv).and_then(|()| trust_parse_command(argv)) {
        Ok(command) => trust_execute_command(&command),
        Err(message) if message.is_empty() => trust_ok(TRUST_USAGE),
        Err(message) => trust_error(&message),
    }
}

fn trust_parse_command(argv: &[String]) -> Result<TrustCommandPlan, String> {
    trust_validate_separator(argv)?;
    let Some(subcommand) = argv.first() else {
        return Err(String::new());
    };
    if subcommand == "help" || subcommand == "--help" || subcommand == "-h" {
        return Err(String::new());
    }
    if subcommand.starts_with('-') {
        return Err("trust subcommand must not start with '-'".to_owned());
    }
    match subcommand.as_str() {
        "list" | "ls" => trust_parse_list(argv),
        "add" => trust_parse_add(argv),
        "remove" | "rm" | "delete" => trust_parse_remove(argv),
        _ => Err(format!(
            "maw trust: unknown subcommand \"{subcommand}\" (expected list|add|remove)"
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrustCommandPlan {
    List,
    Add { sender: String, target: String },
    Remove {
        sender: String,
        target: String,
        yes: bool,
    },
}

fn trust_parse_list(argv: &[String]) -> Result<TrustCommandPlan, String> {
    if argv.len() == 1 {
        Ok(TrustCommandPlan::List)
    } else {
        Err("trust list: unexpected argument".to_owned())
    }
}

fn trust_parse_add(argv: &[String]) -> Result<TrustCommandPlan, String> {
    if argv.len() != 3 {
        return Err("trust add: expected <sender> <target>".to_owned());
    }
    let sender = trust_validate_actor("sender", &argv[1])?;
    let target = trust_validate_actor("target", &argv[2])?;
    trust_validate_not_self(&sender, &target)?;
    Ok(TrustCommandPlan::Add { sender, target })
}

fn trust_parse_remove(argv: &[String]) -> Result<TrustCommandPlan, String> {
    if argv.len() != 3 && argv.len() != 4 {
        return Err("trust remove: expected <sender> <target> [--yes]".to_owned());
    }
    let sender = trust_validate_actor("sender", &argv[1])?;
    let target = trust_validate_actor("target", &argv[2])?;
    trust_validate_not_self(&sender, &target)?;
    let yes = argv.get(3).is_some_and(|flag| flag == "--yes" || flag == "-y");
    if argv.get(3).is_some_and(|flag| flag != "--yes" && flag != "-y") {
        return Err("trust remove: only --yes is supported as a flag".to_owned());
    }
    Ok(TrustCommandPlan::Remove {
        sender,
        target,
        yes,
    })
}

fn trust_execute_command(command: &TrustCommandPlan) -> CliOutput {
    match command {
        TrustCommandPlan::List => trust_ok(&trust_format_list(&trust_load_fake_entries())),
        TrustCommandPlan::Add { sender, target } => trust_execute_add(sender, target),
        TrustCommandPlan::Remove {
            sender,
            target,
            yes,
        } => trust_execute_remove(sender, target, *yes),
    }
}

fn trust_execute_add(sender: &str, target: &str) -> CliOutput {
    let entries = trust_load_fake_entries();
    if let Some(entry) = trust_find_entry(&entries, sender, target) {
        return trust_ok(&format!(
            "already trusted: \"{}\" ↔ \"{}\" (added {})",
            entry.sender, entry.target, entry.added_at
        ));
    }
    CliOutput {
        code: 0,
        stdout: format!("plan: would trust \"{sender}\" ↔ \"{target}\"\n"),
        stderr: TRUST_STUB_WARNING.to_owned(),
    }
}

fn trust_execute_remove(sender: &str, target: &str, yes: bool) -> CliOutput {
    if !yes {
        return CliOutput {
            code: 2,
            stdout: String::new(),
            stderr: format!(
                "refusing to remove trust relationship without --yes\n  to confirm: maw trust remove {sender} {target} --yes\n"
            ),
        };
    }
    let entries = trust_load_fake_entries();
    if trust_find_entry(&entries, sender, target).is_none() {
        return trust_error("trust remove: no entry found for requested sender/target");
    }
    CliOutput {
        code: 0,
        stdout: format!("plan: would remove trust relationship \"{sender}\" ↔ \"{target}\"\n"),
        stderr: TRUST_STUB_WARNING.to_owned(),
    }
}

fn trust_validate_auto_surface(argv: &[String]) -> Result<(), String> {
    for arg in argv {
        let token = trust_normalize_token(arg);
        if TRUST_AUTO_BLOCKLIST.iter().any(|blocked| *blocked == token) {
            return Err(TRUST_AUTO_REFUSE.to_owned());
        }
        if token.contains("auto-trust")
            || token.contains("auto-approve")
            || token.contains("trust-all")
            || token.contains("approve-all")
        {
            return Err(TRUST_AUTO_REFUSE.to_owned());
        }
    }
    Ok(())
}

fn trust_validate_separator(argv: &[String]) -> Result<(), String> {
    if argv.iter().any(|arg| arg == "--") {
        return Err("trust: -- separator is not allowed".to_owned());
    }
    Ok(())
}

fn trust_validate_actor(label: &str, value: &str) -> Result<String, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("trust: {label} must be a non-empty exact value"));
    }
    if value.starts_with('-') {
        return Err(format!("trust: {label} must not start with '-'"));
    }
    if value == "--" || value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err(format!("trust: {label} contains a rejected path-like value"));
    }
    if value.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err(format!(
            "trust: {label} must not contain whitespace or control characters"
        ));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '@'))
    {
        return Err(format!("trust: {label} contains unsupported characters"));
    }
    Ok(value.to_owned())
}

fn trust_validate_not_self(sender: &str, target: &str) -> Result<(), String> {
    if sender == target {
        return Err("trust add: refusing self-trust relationship; self-messages are always allowed".to_owned());
    }
    Ok(())
}

fn trust_load_fake_entries() -> Vec<TrustEntryPlan> {
    let Some(path) = std::env::var_os(TRUST_FAKE_STORE_ENV) else {
        return Vec::new();
    };
    let Ok(body) = std::fs::read_to_string(std::path::PathBuf::from(path)) else {
        return Vec::new();
    };
    trust_parse_fake_entries(&body)
}

fn trust_parse_fake_entries(body: &str) -> Vec<TrustEntryPlan> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items.iter().filter_map(trust_entry_from_json).collect()
}

fn trust_entry_from_json(value: &serde_json::Value) -> Option<TrustEntryPlan> {
    let sender = value.get("sender")?.as_str()?.to_owned();
    let target = value.get("target")?.as_str()?.to_owned();
    let added_at = value.get("addedAt")?.as_str()?.to_owned();
    if trust_validate_actor("sender", &sender).is_err()
        || trust_validate_actor("target", &target).is_err()
        || trust_validate_not_self(&sender, &target).is_err()
    {
        return None;
    }
    Some(TrustEntryPlan {
        sender,
        target,
        added_at,
    })
}

fn trust_format_list(entries: &[TrustEntryPlan]) -> String {
    if entries.is_empty() {
        return "no trust entries".to_owned();
    }
    let mut rows = entries.to_vec();
    rows.sort_by(|left, right| left.added_at.cmp(&right.added_at));
    let mut text = String::from("trusted relationships:\n");
    for entry in rows {
        let _ = writeln!(
            text,
            "  {} ↔ {}   added {}",
            entry.sender, entry.target, entry.added_at
        );
    }
    text.trim_end().to_owned()
}

fn trust_find_entry<'a>(
    entries: &'a [TrustEntryPlan],
    sender: &str,
    target: &str,
) -> Option<&'a TrustEntryPlan> {
    entries
        .iter()
        .find(|entry| trust_same_relationship(&entry.sender, &entry.target, sender, target))
}

fn trust_same_relationship(left_sender: &str, left_target: &str, right_sender: &str, right_target: &str) -> bool {
    (left_sender == right_sender && left_target == right_target)
        || (left_sender == right_target && left_target == right_sender)
}

fn trust_normalize_token(value: &str) -> String {
    value
        .trim_start_matches('-')
        .replace('_', "-")
        .to_ascii_lowercase()
}

fn trust_ok(message: &str) -> CliOutput {
    CliOutput {
        code: 0,
        stdout: format!("{message}\n"),
        stderr: String::new(),
    }
}

fn trust_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\n"),
    }
}

#[cfg(test)]
mod trust_native_tests {
    use super::*;

    fn trust_args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn trust_dispatch_registers_native_command_and_alias() {
        let commands = DISPATCH_98
            .iter()
            .map(|entry| entry.command)
            .collect::<Vec<_>>();
        assert_eq!(commands, ["trust", "trusts"]);
        assert_eq!(dispatcher_status("trust"), DispatchKind::Native);
        assert_eq!(dispatcher_status("trusts"), DispatchKind::Native);
    }

    #[test]
    fn trust_blocks_auto_consent_surface_without_secret_echo() {
        let output = trust_run_command(&trust_args(&[
            "add",
            "--auto-trust=fake-secret-token",
            "alpha",
            "beta",
        ]));
        assert_eq!(output.code, 2);
        assert!(output.stderr.contains("no auto-trust"));
        assert!(!output.stderr.contains("fake-secret-token"));

        let blocked = trust_run_command(&trust_args(&["trust-all", "alpha", "beta"]));
        assert_eq!(blocked.code, 2);
        assert!(blocked.stderr.contains("explicit human flow"));
    }

    #[test]
    fn trust_guards_separator_and_leading_dash_without_echo() {
        let sep = trust_run_command(&trust_args(&["add", "alpha", "--", "beta"]));
        assert_eq!(sep.code, 2);
        assert!(sep.stderr.contains("separator"));
        let guarded = trust_run_command(&trust_args(&["add", "-secret-token", "beta"]));
        assert_eq!(guarded.code, 2);
        assert!(guarded.stderr.contains("sender must not start"));
        assert!(!guarded.stderr.contains("secret-token"));
    }

    #[test]
    fn trust_parses_fake_entries_and_lists_sorted() {
        let entries = trust_parse_fake_entries(
            r#"[
              {"sender":"beta","target":"alpha","addedAt":"2026-06-10T00:00:00.000Z"},
              {"sender":"gamma","target":"delta","addedAt":"2026-06-09T00:00:00.000Z"},
              {"sender":"bad/name","target":"delta","addedAt":"2026-06-08T00:00:00.000Z"}
            ]"#,
        );
        assert_eq!(entries.len(), 2);
        assert!(trust_find_entry(&entries, "alpha", "beta").is_some());
        let list = trust_format_list(&entries);
        let gamma = list.find("gamma ↔ delta").expect("gamma row");
        let beta = list.find("beta ↔ alpha").expect("beta row");
        assert!(gamma < beta, "{list}");
    }

    #[test]
    fn trust_add_and_remove_are_plan_only() {
        let add = trust_run_command(&trust_args(&["add", "alpha", "beta"]));
        assert_eq!(add.code, 0, "{}", add.stderr);
        assert!(add.stdout.contains("plan: would trust"));
        assert!(add.stderr.contains("TODO(#115)"));
        let remove = trust_run_command(&trust_args(&["remove", "alpha", "beta"]));
        assert_eq!(remove.code, 2);
        assert!(remove.stderr.contains("without --yes"));
    }

    #[test]
    fn trust_rejects_self_and_control_values() {
        let self_trust = trust_run_command(&trust_args(&["add", "alpha", "alpha"]));
        assert_eq!(self_trust.code, 2);
        assert!(self_trust.stderr.contains("self-trust"));
        let control = trust_run_command(&trust_args(&["add", "alpha\nbeta", "gamma"]));
        assert_eq!(control.code, 2);
        assert!(control.stderr.contains("control"));
        assert!(!control.stderr.contains("alpha\nbeta"));
    }
}
