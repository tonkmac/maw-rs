const DISPATCH_78: &[DispatcherEntry] = &[DispatcherEntry {
    command: "kill",
    handler: Handler::Sync(kill_run_command),
}];

const KILL_USAGE: &str = "usage: maw kill <target>[:window] [--pane N] [--index N|--all] [--peer <alias>]  (see: maw sleep for graceful stop, maw done for worktrees)";
const KILL_WINDOW_FORMAT: &str =
    "#{session_name}|||#{window_index}|||#{window_name}|||#{window_active}|||#{pane_current_path}";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct KillOptions {
    target: String,
    pane: Option<u32>,
    index: Option<u32>,
    all: bool,
    peer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KillSession {
    name: String,
    windows: Vec<KillWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KillWindow {
    index: u32,
    name: String,
}

trait KillTmux {
    fn kill_list_sessions(&mut self) -> Result<Vec<KillSession>, String>;
    fn kill_list_panes_all(&mut self) -> Result<String, String>;
    fn kill_list_pane_indexes(&mut self, target: &str) -> Result<Vec<u32>, String>;
    fn kill_kill_session(&mut self, session: &str) -> Result<(), String>;
    fn kill_kill_window(&mut self, target: &str) -> Result<(), String>;
    fn kill_kill_pane(&mut self, target: &str) -> Result<(), String>;
}

struct KillSystemTmux {
    runner: maw_tmux::CommandTmuxRunner,
}

impl KillSystemTmux {
    fn kill_new() -> Self {
        Self {
            runner: maw_tmux::CommandTmuxRunner::new(),
        }
    }
}

impl KillTmux for KillSystemTmux {
    fn kill_list_sessions(&mut self) -> Result<Vec<KillSession>, String> {
        kill_tmux_run(
            &mut self.runner,
            "list-windows",
            &["-a", "-F", KILL_WINDOW_FORMAT],
        )
        .map(|raw| kill_parse_sessions(&raw))
    }

    fn kill_list_panes_all(&mut self) -> Result<String, String> {
        kill_tmux_run(
            &mut self.runner,
            "list-panes",
            &["-a", "-F", maw_tmux::PANE_TARGET_FORMAT],
        )
    }

    fn kill_list_pane_indexes(&mut self, target: &str) -> Result<Vec<u32>, String> {
        kill_validate_tmux_target(target)?;
        kill_tmux_run(
            &mut self.runner,
            "list-panes",
            &["-t", target, "-F", "#{pane_index}"],
        )
        .map(|raw| kill_parse_numbers(&raw))
    }

    fn kill_kill_session(&mut self, session: &str) -> Result<(), String> {
        kill_validate_tmux_target(session)?;
        kill_tmux_run(&mut self.runner, "kill-session", &["-t", session]).map(|_| ())
    }

    fn kill_kill_window(&mut self, target: &str) -> Result<(), String> {
        kill_validate_tmux_target(target)?;
        kill_tmux_run(&mut self.runner, "kill-window", &["-t", target]).map(|_| ())
    }

    fn kill_kill_pane(&mut self, target: &str) -> Result<(), String> {
        kill_validate_tmux_target(target)?;
        kill_tmux_run(&mut self.runner, "kill-pane", &["-t", target]).map(|_| ())
    }
}

fn kill_run_command(argv: &[String]) -> CliOutput {
    if kill_has_peer_flag(argv) {
        let mut fallback_argv = vec!["kill".to_owned()];
        fallback_argv.extend(argv.iter().cloned());
        return dispatch_bun_fallback(&fallback_argv, "kill");
    }
    kill_run_command_with(argv, &mut KillSystemTmux::kill_new())
}

fn kill_has_peer_flag(argv: &[String]) -> bool {
    argv.iter()
        .any(|arg| arg == "--peer" || arg.starts_with("--peer="))
}

fn kill_run_command_with(argv: &[String], tmux: &mut impl KillTmux) -> CliOutput {
    match kill_run(argv, tmux) {
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

fn kill_run(argv: &[String], tmux: &mut impl KillTmux) -> Result<String, String> {
    let options = kill_parse_args(argv)?;
    if let Some(peer) = &options.peer {
        return Err(format!(
            "peer kill is not available in the native port yet: {peer}"
        ));
    }
    kill_validate_user_target(&options.target)?;
    let (raw_session, raw_window) = kill_split_target(&options.target);
    kill_validate_user_target(&raw_session)?;
    let sessions = tmux.kill_list_sessions()?;
    kill_resolve_and_apply(tmux, &sessions, &raw_session, &raw_window, &options)
}

fn kill_parse_args(argv: &[String]) -> Result<KillOptions, String> {
    let mut options = KillOptions::default();
    let mut index = 0;
    while index < argv.len() {
        index += kill_parse_arg(argv, index, &mut options)?;
    }
    if options.target.is_empty() || options.target == "--help" || options.target == "-h" {
        return Err(KILL_USAGE.to_owned());
    }
    Ok(options)
}

fn kill_parse_arg(
    argv: &[String],
    index: usize,
    options: &mut KillOptions,
) -> Result<usize, String> {
    let arg = argv[index].as_str();
    match arg {
        "--all" => {
            options.all = true;
            Ok(1)
        }
        "--pane" => kill_parse_value_flag(argv, index, "--pane", |value| {
            options.pane = Some(kill_parse_non_negative(value, "--pane")?);
            Ok(())
        }),
        "--index" => kill_parse_value_flag(argv, index, "--index", |value| {
            options.index = Some(kill_parse_non_negative(value, "--index")?);
            Ok(())
        }),
        "--peer" => kill_parse_value_flag(argv, index, "--peer", |value| {
            kill_validate_user_target(value)?;
            options.peer = Some(value.to_owned());
            Ok(())
        }),
        value if value.starts_with("--pane=") => {
            options.pane = Some(kill_parse_non_negative(&value[7..], "--pane")?);
            Ok(1)
        }
        value if value.starts_with("--index=") => {
            options.index = Some(kill_parse_non_negative(&value[8..], "--index")?);
            Ok(1)
        }
        value if value.starts_with("--peer=") => {
            kill_validate_user_target(&value[7..])?;
            options.peer = Some(value[7..].to_owned());
            Ok(1)
        }
        value if value.starts_with('-') => Err(format!(
            "\"{value}\" looks like a flag, not a target.\n  usage: maw kill <target>  (see: maw sleep for graceful stop, maw done for worktrees)"
        )),
        value => {
            if !options.target.is_empty() {
                return Err(format!("kill: unexpected argument {value}"));
            }
            value.clone_into(&mut options.target);
            Ok(1)
        }
    }
}

fn kill_parse_value_flag<F>(
    argv: &[String],
    index: usize,
    flag: &str,
    mut assign: F,
) -> Result<usize, String>
where
    F: FnMut(&str) -> Result<(), String>,
{
    let value = argv
        .get(index + 1)
        .ok_or_else(|| format!("kill: missing {flag} value"))?;
    if value.starts_with('-') {
        return Err(format!("kill: {flag} value must not start with '-'"));
    }
    assign(value)?;
    Ok(2)
}

fn kill_parse_non_negative(value: &str, flag: &str) -> Result<u32, String> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!(
            "{flag} must be a non-negative integer (got {value})"
        ));
    }
    value
        .parse::<u32>()
        .map_err(|_| format!("{flag} must be a non-negative integer (got {value})"))
}

fn kill_split_target(target: &str) -> (String, String) {
    target.split_once(':').map_or_else(
        || (target.to_owned(), String::new()),
        |(session, window)| (session.to_owned(), window.to_owned()),
    )
}

fn kill_resolve_and_apply(
    tmux: &mut impl KillTmux,
    sessions: &[KillSession],
    raw_session: &str,
    raw_window: &str,
    options: &KillOptions,
) -> Result<String, String> {
    let names = sessions
        .iter()
        .map(|session| session.name.clone())
        .collect::<Vec<_>>();
    match resolve_session_target(raw_session, &names) {
        ResolveResult::Exact { matched } | ResolveResult::Fuzzy { matched } => {
            let session = kill_find_session(sessions, &matched)?;
            kill_apply_resolved(tmux, session, raw_window, options)
        }
        ResolveResult::Ambiguous { candidates } => Err(kill_ambiguous_session(
            raw_session,
            &kill_sessions_for_names(sessions, &candidates),
        )),
        ResolveResult::None { hints } => {
            let hint_sessions = hints.map(|names| kill_sessions_for_names(sessions, &names));
            kill_apply_orphan_pane_fallback(
                tmux,
                raw_session,
                raw_window,
                options,
                hint_sessions.as_deref(),
            )
        }
    }
}

fn kill_find_session<'a>(
    sessions: &'a [KillSession],
    name: &str,
) -> Result<&'a KillSession, String> {
    sessions
        .iter()
        .find(|session| session.name == name)
        .ok_or_else(|| format!("session '{name}' not found after resolution"))
}

fn kill_sessions_for_names(sessions: &[KillSession], names: &[String]) -> Vec<KillSession> {
    names
        .iter()
        .filter_map(|name| sessions.iter().find(|session| session.name == *name))
        .cloned()
        .collect()
}

fn kill_apply_resolved(
    tmux: &mut impl KillTmux,
    session: &KillSession,
    raw_window: &str,
    options: &KillOptions,
) -> Result<String, String> {
    kill_validate_tmux_target(&session.name)?;
    let indexes = kill_matching_window_indexes(session, raw_window, options)?;
    if let Some(pane) = options.pane {
        return kill_kill_resolved_pane(tmux, session, indexes.first().copied(), pane);
    }
    if raw_window.is_empty() && options.index.is_none() && !options.all {
        tmux.kill_kill_session(&session.name)?;
        return Ok(format!(
            "  \x1b[32m✓\x1b[0m killed session {}\n",
            session.name
        ));
    }
    kill_kill_resolved_windows(tmux, session, &indexes, options)
}

fn kill_apply_orphan_pane_fallback(
    tmux: &mut impl KillTmux,
    raw_session: &str,
    raw_window: &str,
    options: &KillOptions,
    hints: Option<&[KillSession]>,
) -> Result<String, String> {
    if raw_window.is_empty() && options.pane.is_none() {
        let pane_raw = tmux.kill_list_panes_all().unwrap_or_default();
        if !pane_raw.trim().is_empty() {
            return kill_resolve_orphan_pane(tmux, raw_session, &pane_raw);
        }
    }
    Err(kill_missing_session(raw_session, hints))
}

fn kill_resolve_orphan_pane(
    tmux: &mut impl KillTmux,
    raw_session: &str,
    pane_raw: &str,
) -> Result<String, String> {
    match maw_tmux::resolve_pane_target_from_list_panes_output(raw_session, pane_raw) {
        maw_tmux::PaneTargetResolution::Match { candidate } => {
            kill_validate_tmux_target(&candidate.resolved)?;
            tmux.kill_kill_pane(&candidate.resolved)?;
            Ok(format!(
                "  \x1b[32m✓\x1b[0m killed pane {raw_session} → {} \x1b[90m[{} ({})]\x1b[0m\n",
                candidate.resolved, candidate.source, candidate.name
            ))
        }
        maw_tmux::PaneTargetResolution::Ambiguous { candidates } => {
            Err(kill_ambiguous_panes(raw_session, &candidates))
        }
        maw_tmux::PaneTargetResolution::None => Err(kill_missing_session(raw_session, None)),
    }
}

fn kill_matching_window_indexes(
    session: &KillSession,
    raw_window: &str,
    options: &KillOptions,
) -> Result<Vec<u32>, String> {
    if options.all && options.index.is_some() {
        return Err("cannot combine --all and --index".to_owned());
    }
    if options.all && options.pane.is_some() {
        return Err("cannot combine --all and --pane".to_owned());
    }
    if let Some(index) = options.index {
        kill_require_window_index(session, index)?;
        return Ok(vec![index]);
    }
    if raw_window.is_empty() {
        return Ok(Vec::new());
    }
    if raw_window.chars().all(|ch| ch.is_ascii_digit()) {
        let index = kill_parse_non_negative(raw_window, "window index")?;
        kill_require_window_index(session, index)?;
        return Ok(vec![index]);
    }
    let matches = session
        .windows
        .iter()
        .filter(|window| window.name.eq_ignore_ascii_case(raw_window))
        .map(|window| window.index)
        .collect::<Vec<_>>();
    kill_validate_window_matches(session, raw_window, &matches, options.all)
}

fn kill_validate_window_matches(
    session: &KillSession,
    raw_window: &str,
    matches: &[u32],
    all: bool,
) -> Result<Vec<u32>, String> {
    if matches.is_empty() {
        return Err(format!(
            "window '{raw_window}' not found in session {} (valid: {})",
            session.name,
            kill_window_labels(session)
        ));
    }
    if matches.len() > 1 && !all {
        return Err(kill_ambiguous_window(session, raw_window, matches));
    }
    Ok(matches.to_vec())
}

fn kill_require_window_index(session: &KillSession, index: u32) -> Result<(), String> {
    if session.windows.iter().any(|window| window.index == index) {
        Ok(())
    } else {
        Err(format!(
            "window index {index} does not exist in session {} (valid: {})",
            session.name,
            kill_window_labels(session)
        ))
    }
}

fn kill_kill_resolved_pane(
    tmux: &mut impl KillTmux,
    session: &KillSession,
    window_index: Option<u32>,
    pane_index: u32,
) -> Result<String, String> {
    let win =
        window_index.unwrap_or_else(|| session.windows.first().map_or(0, |window| window.index));
    kill_require_window_index(session, win)?;
    let win_target = format!("{}:{win}", session.name);
    kill_validate_tmux_target(&win_target)?;
    let valid = tmux.kill_list_pane_indexes(&win_target)?;
    if !valid.contains(&pane_index) {
        let list = kill_number_list(&valid);
        return Err(format!(
            "pane {pane_index} does not exist in window {win_target} (valid: {list})"
        ));
    }
    let pane = format!("{win_target}.{pane_index}");
    kill_validate_tmux_target(&pane)?;
    tmux.kill_kill_pane(&pane)?;
    Ok(format!("  \x1b[32m✓\x1b[0m killed pane {pane}\n"))
}

fn kill_kill_resolved_windows(
    tmux: &mut impl KillTmux,
    session: &KillSession,
    indexes: &[u32],
    options: &KillOptions,
) -> Result<String, String> {
    if indexes.is_empty() {
        return Err(if options.all {
            "--all requires a window name target (session:window)".to_owned()
        } else {
            "window target required".to_owned()
        });
    }
    let mut killed = Vec::new();
    for index in indexes {
        let target = format!("{}:{index}", session.name);
        kill_validate_tmux_target(&target)?;
        tmux.kill_kill_window(&target)?;
        killed.push(target);
    }
    Ok(kill_window_success(&killed))
}

fn kill_window_success(killed: &[String]) -> String {
    if killed.len() == 1 {
        format!("  \x1b[32m✓\x1b[0m killed window {}\n", killed[0])
    } else {
        format!(
            "  \x1b[32m✓\x1b[0m killed {} windows {}\n",
            killed.len(),
            killed.join(", ")
        )
    }
}

fn kill_parse_sessions(raw: &str) -> Vec<KillSession> {
    let mut sessions = Vec::<KillSession>::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        kill_push_window(&mut sessions, line);
    }
    sessions
}

fn kill_push_window(sessions: &mut Vec<KillSession>, line: &str) {
    let parts = line.split("|||").collect::<Vec<_>>();
    let name = parts.first().copied().unwrap_or_default().to_owned();
    let index = parts
        .get(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let window = KillWindow {
        index,
        name: parts.get(2).copied().unwrap_or_default().to_owned(),
    };
    if let Some(session) = sessions.iter_mut().find(|session| session.name == name) {
        session.windows.push(window);
    } else {
        sessions.push(KillSession {
            name,
            windows: vec![window],
        });
    }
}

fn kill_parse_numbers(raw: &str) -> Vec<u32> {
    raw.lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect()
}

fn kill_tmux_run<R: maw_tmux::TmuxRunner>(
    runner: &mut R,
    subcommand: &str,
    args: &[&str],
) -> Result<String, String> {
    let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    runner.run(subcommand, &args).map_err(|error| error.message)
}

fn kill_validate_user_target(target: &str) -> Result<(), String> {
    if target.is_empty()
        || target.trim() != target
        || target.starts_with('-')
        || target.contains('\0')
    {
        Err("kill target must be non-empty, unpadded, and not start with '-'".to_owned())
    } else {
        Ok(())
    }
}

fn kill_validate_tmux_target(target: &str) -> Result<(), String> {
    if target.is_empty()
        || target.trim() != target
        || target.starts_with('-')
        || target.contains('\0')
    {
        Err("tmux target/session must be non-empty, unpadded, and not start with '-'".to_owned())
    } else {
        Ok(())
    }
}

fn kill_window_labels(session: &KillSession) -> String {
    if session.windows.is_empty() {
        return "(none)".to_owned();
    }
    session
        .windows
        .iter()
        .map(|window| format!("{}:{}", window.index, window.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn kill_number_list(values: &[u32]) -> String {
    if values.is_empty() {
        "(none)".to_owned()
    } else {
        values
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn kill_ambiguous_session(target: &str, candidates: &[KillSession]) -> String {
    let mut out = format!(
        "  \x1b[31m✗\x1b[0m '{target}' is ambiguous — matches {} sessions:",
        candidates.len()
    );
    for session in candidates {
        let _ = write!(out, "\n  \x1b[90m    • {}\x1b[0m", session.name);
    }
    out.push_str("\n  \x1b[90m  use the full name: maw kill <exact-session>\x1b[0m");
    out
}

fn kill_missing_session(target: &str, hints: Option<&[KillSession]>) -> String {
    let mut out = format!("  \x1b[31m✗\x1b[0m session '{target}' not found");
    if let Some(hints) = hints.filter(|hints| !hints.is_empty()) {
        out.push_str("\n  \x1b[90m  did you mean:\x1b[0m");
        for session in hints {
            let _ = write!(out, "\n  \x1b[90m    • {}\x1b[0m", session.name);
        }
    } else {
        out.push_str("\n  \x1b[90m  try: maw ls\x1b[0m");
    }
    out
}

fn kill_ambiguous_window(session: &KillSession, raw_window: &str, matches: &[u32]) -> String {
    let mut out = format!(
        "window '{raw_window}' is ambiguous in session {} — matches {} windows:",
        session.name,
        matches.len()
    );
    for index in matches {
        if let Some(window) = session.windows.iter().find(|window| window.index == *index) {
            let _ = write!(out, "\n    • {}:{}", window.index, window.name);
        }
    }
    out.push_str("\n  use --index N to kill one, or --all to kill all matching windows");
    out
}

fn kill_ambiguous_panes(target: &str, candidates: &[maw_tmux::PaneTargetCandidate]) -> String {
    let mut out = format!(
        "  \x1b[31m✗\x1b[0m '{target}' is ambiguous — matches {} panes:",
        candidates.len()
    );
    for candidate in candidates {
        let _ = write!(
            out,
            "\n  \x1b[90m    • {} → {} ({}) [{}]\x1b[0m",
            candidate.name, candidate.resolved, candidate.target, candidate.source
        );
    }
    out.push_str("\n  \x1b[90m  use the pane id or full session:window.pane target\x1b[0m");
    out
}

#[cfg(test)]
mod kill_tests {
    use super::*;

    #[derive(Debug, Default)]
    struct KillFakeTmux {
        sessions_raw: String,
        panes_all_raw: String,
        pane_indexes_raw: String,
        calls: Vec<(String, Vec<String>)>,
        fail_kill: Option<String>,
    }

    impl KillTmux for KillFakeTmux {
        fn kill_list_sessions(&mut self) -> Result<Vec<KillSession>, String> {
            self.calls.push((
                "list-windows".to_owned(),
                kill_strings(&["-a", "-F", KILL_WINDOW_FORMAT]),
            ));
            Ok(kill_parse_sessions(&self.sessions_raw))
        }

        fn kill_list_panes_all(&mut self) -> Result<String, String> {
            self.calls.push((
                "list-panes".to_owned(),
                kill_strings(&["-a", "-F", maw_tmux::PANE_TARGET_FORMAT]),
            ));
            Ok(self.panes_all_raw.clone())
        }

        fn kill_list_pane_indexes(&mut self, target: &str) -> Result<Vec<u32>, String> {
            kill_validate_tmux_target(target)?;
            self.calls.push((
                "list-panes".to_owned(),
                kill_strings(&["-t", target, "-F", "#{pane_index}"]),
            ));
            Ok(kill_parse_numbers(&self.pane_indexes_raw))
        }

        fn kill_kill_session(&mut self, session: &str) -> Result<(), String> {
            kill_validate_tmux_target(session)?;
            self.calls
                .push(("kill-session".to_owned(), kill_strings(&["-t", session])));
            kill_maybe_fail(self.fail_kill.as_ref())
        }

        fn kill_kill_window(&mut self, target: &str) -> Result<(), String> {
            kill_validate_tmux_target(target)?;
            self.calls
                .push(("kill-window".to_owned(), kill_strings(&["-t", target])));
            kill_maybe_fail(self.fail_kill.as_ref())
        }

        fn kill_kill_pane(&mut self, target: &str) -> Result<(), String> {
            kill_validate_tmux_target(target)?;
            self.calls
                .push(("kill-pane".to_owned(), kill_strings(&["-t", target])));
            kill_maybe_fail(self.fail_kill.as_ref())
        }
    }

    fn kill_maybe_fail(error: Option<&String>) -> Result<(), String> {
        error.cloned().map_or(Ok(()), Err)
    }

    fn kill_strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn kill_fake(sessions_raw: &str) -> KillFakeTmux {
        KillFakeTmux {
            sessions_raw: sessions_raw.to_owned(),
            ..KillFakeTmux::default()
        }
    }

    #[test]
    fn kill_dispatch_registers_native_kill() {
        assert_eq!(DISPATCH_78.len(), 1);
        assert_eq!(DISPATCH_78[0].command, "kill");
    }

    #[test]
    fn kill_session_resolves_and_validates_before_destructive_call() {
        let mut tmux = kill_fake("07-demo|||0|||main|||1|||/tmp\n");
        let output = kill_run_command_with(&kill_strings(&["demo"]), &mut tmux);
        assert_eq!(output.code, 0);
        assert_eq!(output.stdout, "  \x1b[32m✓\x1b[0m killed session 07-demo\n");
        assert_eq!(tmux.calls[0].0, "list-windows");
        assert_eq!(
            tmux.calls[1],
            ("kill-session".to_owned(), kill_strings(&["-t", "07-demo"]))
        );
    }

    #[test]
    fn kill_rejects_leading_dash_target_before_listing_or_kill() {
        let mut tmux = kill_fake("07-demo|||0|||main|||1|||/tmp\n");
        let output = kill_run_command_with(&kill_strings(&["-Sbad"]), &mut tmux);
        assert_eq!(output.code, 1);
        assert!(output.stderr.contains("looks like a flag"));
        assert!(tmux.calls.is_empty());
    }

    #[test]
    fn kill_refuses_invalid_resolved_session_before_destructive_call() {
        let mut tmux = kill_fake("-Sbad-demo|||0|||main|||1|||/tmp\n");
        let output = kill_run_command_with(&kill_strings(&["demo"]), &mut tmux);
        assert_eq!(output.code, 1);
        assert!(output.stderr.contains("target/session"));
        assert_eq!(
            tmux.calls.len(),
            1,
            "listed before refusing resolved kill target"
        );
    }

    #[test]
    fn kill_window_index_and_all_are_validated_against_listing() {
        let mut tmux = kill_fake("07-demo|||0|||work|||1|||/tmp\n07-demo|||2|||work|||0|||/tmp\n");
        let output = kill_run_command_with(&kill_strings(&["07-demo:work", "--all"]), &mut tmux);
        assert_eq!(output.code, 0);
        assert!(output.stdout.contains("killed 2 windows"));
        assert_eq!(
            tmux.calls[1],
            ("kill-window".to_owned(), kill_strings(&["-t", "07-demo:0"]))
        );
        assert_eq!(
            tmux.calls[2],
            ("kill-window".to_owned(), kill_strings(&["-t", "07-demo:2"]))
        );
    }

    #[test]
    fn kill_ambiguous_window_requires_index_or_all_and_does_not_kill() {
        let mut tmux = kill_fake("07-demo|||0|||work|||1|||/tmp\n07-demo|||2|||work|||0|||/tmp\n");
        let output = kill_run_command_with(&kill_strings(&["07-demo:work"]), &mut tmux);
        assert_eq!(output.code, 1);
        assert!(output.stderr.contains("ambiguous"));
        assert_eq!(tmux.calls.len(), 1);
    }

    #[test]
    fn kill_pane_lists_valid_indexes_before_kill_pane() {
        let mut tmux = kill_fake("07-demo|||1|||main|||1|||/tmp\n");
        tmux.pane_indexes_raw = "0\n2\n".to_owned();
        let output = kill_run_command_with(&kill_strings(&["demo:1", "--pane", "2"]), &mut tmux);
        assert_eq!(output.code, 0);
        assert_eq!(
            output.stdout,
            "  \x1b[32m✓\x1b[0m killed pane 07-demo:1.2\n"
        );
        assert_eq!(
            tmux.calls[1],
            (
                "list-panes".to_owned(),
                kill_strings(&["-t", "07-demo:1", "-F", "#{pane_index}"])
            )
        );
        assert_eq!(
            tmux.calls[2],
            ("kill-pane".to_owned(), kill_strings(&["-t", "07-demo:1.2"]))
        );
    }

    #[test]
    fn kill_pane_rejects_missing_pane_without_kill() {
        let mut tmux = kill_fake("07-demo|||1|||main|||1|||/tmp\n");
        tmux.pane_indexes_raw = "0\n".to_owned();
        let output = kill_run_command_with(&kill_strings(&["demo:1", "--pane=2"]), &mut tmux);
        assert_eq!(output.code, 1);
        assert!(output.stderr.contains("pane 2 does not exist"));
        assert!(!tmux.calls.iter().any(|call| call.0 == "kill-pane"));
    }

    #[test]
    fn kill_orphan_pane_fallback_uses_pane_resolver_before_kill() {
        let mut tmux = kill_fake("");
        tmux.panes_all_raw = "%42|||07-demo:1.0|||agent|||role|||/repo/demo\n".to_owned();
        let output = kill_run_command_with(&kill_strings(&["agent"]), &mut tmux);
        assert_eq!(output.code, 0);
        assert!(output.stdout.contains("killed pane agent → %42"));
        assert_eq!(tmux.calls[0].0, "list-windows");
        assert_eq!(tmux.calls[1].0, "list-panes");
        assert_eq!(
            tmux.calls[2],
            ("kill-pane".to_owned(), kill_strings(&["-t", "%42"]))
        );
    }

    #[test]
    fn kill_missing_session_prints_hints_and_does_not_kill() {
        let mut tmux = kill_fake("07-demo|||0|||main|||1|||/tmp\n");
        let output = kill_run_command_with(&kill_strings(&["dem"]), &mut tmux);
        assert_eq!(output.code, 1);
        assert!(output.stderr.contains("did you mean"));
        assert!(!tmux.calls.iter().any(|call| call.0.starts_with("kill-")));
    }

    #[test]
    fn kill_rejects_bad_flag_combinations_before_kill() {
        let mut tmux = kill_fake("07-demo|||0|||main|||1|||/tmp\n");
        let output = kill_run_command_with(
            &kill_strings(&["demo:main", "--all", "--pane", "0"]),
            &mut tmux,
        );
        assert_eq!(output.code, 1);
        assert!(output.stderr.contains("cannot combine --all and --pane"));
        assert_eq!(tmux.calls.len(), 1);
    }
}
