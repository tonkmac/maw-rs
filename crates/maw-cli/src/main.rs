use std::{
    collections::BTreeSet,
    io::IsTerminal,
    process::{Command, Stdio},
};

use maw_tmux::{resolve_tmux_attach_session, TmuxAttachSessionResolution, TmuxClient};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = maybe_exec_attach(&argv) {
        std::process::exit(code);
    }
    let output = maw_cli::run_cli(&argv);
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    std::process::exit(output.code);
}

fn maybe_exec_attach(argv: &[String]) -> Option<i32> {
    let mut client = TmuxClient::local();
    let alive_sessions = client.list_session_names();
    maybe_exec_attach_with(
        argv,
        std::io::stdout().is_terminal(),
        std::env::var_os("TMUX").is_some(),
        &alive_sessions,
        run_tmux_attach,
    )
}

fn maybe_exec_attach_with(
    argv: &[String],
    stdout_is_terminal: bool,
    inside_tmux: bool,
    alive_sessions: &[String],
    run: impl FnOnce(Vec<String>) -> i32,
) -> Option<i32> {
    attach_exec_tmux_args(argv, stdout_is_terminal, inside_tmux, alive_sessions).map(run)
}

fn run_tmux_attach(tmux_args: Vec<String>) -> i32 {
    let status = Command::new("tmux")
        .args(tmux_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("attach: failed to execute tmux: {error}");
            1
        }
    }
}

fn attach_exec_tmux_args(
    argv: &[String],
    stdout_is_terminal: bool,
    inside_tmux: bool,
    alive_sessions: &[String],
) -> Option<Vec<String>> {
    let verb = argv.first()?.as_str();
    if !matches!(verb, "a" | "attach") {
        return None;
    }
    if argv.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--help" | "-h" | "--print" | "--plan-json" | "--dry-run"
        )
    }) {
        return None;
    }
    if !stdout_is_terminal {
        return None;
    }

    let mut readonly = false;
    let mut target: Option<&str> = None;
    for arg in argv.iter().skip(1).map(String::as_str) {
        match arg {
            "--readonly" | "--read-only" | "-r" => readonly = true,
            arg if arg.starts_with('-') => return None,
            value => {
                if target.is_some() {
                    return None;
                }
                target = Some(value);
            }
        }
    }
    let session_query = target?.split(':').next().unwrap_or_default();
    let alive = alive_sessions.iter().cloned().collect::<BTreeSet<_>>();
    let session = match resolve_tmux_attach_session(session_query, &alive) {
        TmuxAttachSessionResolution::Match { session } => session,
        TmuxAttachSessionResolution::Ambiguous { .. }
        | TmuxAttachSessionResolution::Missing { .. } => return None,
    };
    let tmux_args = if readonly {
        vec![
            "attach".to_owned(),
            "-r".to_owned(),
            "-t".to_owned(),
            session,
        ]
    } else if inside_tmux {
        vec!["switch-client".to_owned(), "-t".to_owned(), session]
    } else {
        vec!["attach".to_owned(), "-t".to_owned(), session]
    };
    Some(tmux_args)
}

#[cfg(test)]
mod tests {
    use super::{attach_exec_tmux_args, maybe_exec_attach_with};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn attach_exec_fast_path_rejects_non_live_cli_inputs() {
        assert_eq!(
            attach_exec_tmux_args(&args(&["ls"]), true, false, &args(&["50-mawjs"])),
            None
        );
        assert_eq!(
            attach_exec_tmux_args(&args(&["a", "target"]), false, false, &args(&["target"])),
            None
        );
        assert_eq!(
            attach_exec_tmux_args(
                &args(&["a", "target", "--print"]),
                true,
                false,
                &args(&["target"])
            ),
            None
        );
        assert_eq!(
            attach_exec_tmux_args(&args(&["a", "--unknown"]), true, false, &args(&["target"])),
            None
        );
        assert_eq!(
            attach_exec_tmux_args(
                &args(&["attach", "one", "two"]),
                true,
                false,
                &args(&["one"])
            ),
            None
        );
        assert_eq!(
            attach_exec_tmux_args(&args(&["a"]), true, false, &args(&["50-mawjs"])),
            None
        );
        assert_eq!(
            attach_exec_tmux_args(&args(&["a", "ghost"]), true, false, &args(&["50-mawjs"])),
            None
        );
    }

    #[test]
    fn attach_exec_fast_path_builds_tmux_commands() {
        assert_eq!(
            attach_exec_tmux_args(
                &args(&["a", "mawjs:1.0"]),
                true,
                false,
                &args(&["50-mawjs"])
            ),
            Some(args(&["attach", "-t", "50-mawjs"]))
        );
        assert_eq!(
            attach_exec_tmux_args(
                &args(&["attach", "mawjs"]),
                true,
                true,
                &args(&["50-mawjs"])
            ),
            Some(args(&["switch-client", "-t", "50-mawjs"]))
        );
        assert_eq!(
            attach_exec_tmux_args(
                &args(&["a", "mawjs", "--readonly"]),
                true,
                true,
                &args(&["50-mawjs"])
            ),
            Some(args(&["attach", "-r", "-t", "50-mawjs"]))
        );
        assert_eq!(
            attach_exec_tmux_args(&args(&["a", "volt"]), true, false, &args(&["05-volt"])),
            Some(args(&["attach", "-t", "05-volt"]))
        );
    }

    #[test]
    fn maybe_exec_attach_with_runs_only_valid_fast_path_commands() {
        let code = maybe_exec_attach_with(
            &args(&["a", "mawjs"]),
            true,
            false,
            &args(&["50-mawjs"]),
            |tmux_args| {
                assert_eq!(tmux_args, args(&["attach", "-t", "50-mawjs"]));
                17
            },
        );
        assert_eq!(code, Some(17));

        let blocked = maybe_exec_attach_with(
            &args(&["a", "50-mawjs", "--print"]),
            true,
            false,
            &args(&["50-mawjs"]),
            |_| {
                panic!("print-mode attach must stay in plan mode");
            },
        );
        assert_eq!(blocked, None);
    }
}
