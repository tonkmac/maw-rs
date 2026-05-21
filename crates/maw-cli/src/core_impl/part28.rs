fn run_attach_plan(argv: &[String]) -> CliOutput {
    let mut print = false;
    let mut readonly = false;
    let mut plan_json = false;
    let mut alive = BTreeSet::new();
    let mut target: Option<String> = None;
    let mut index = 0;
    while index < argv.len() {
        match argv[index].as_str() {
            "--help" | "-h" => return attach_usage_ok(),
            "--print" => print = true,
            "--readonly" | "--read-only" | "-r" => readonly = true,
            "--plan-json" | "--dry-run" => plan_json = true,
            "--alive" => {
                let Some(value) = argv.get(index + 1) else {
                    return attach_usage_error("attach: missing --alive value");
                };
                alive.insert(value.to_owned());
                index += 1;
            }
            arg if arg.starts_with("--alive=") => {
                alive.insert(arg["--alive=".len()..].to_owned());
            }
            arg if arg.starts_with('-') => {
                return attach_usage_error(&format!("attach: unknown argument {arg}"));
            }
            value => {
                if target.is_some() {
                    return attach_usage_error("attach: target already provided");
                }
                target = Some(value.to_owned());
            }
        }
        index += 1;
    }

    let Some(target) = target else {
        return attach_usage_error("attach: target required");
    };
    if alive.is_empty() {
        let mut client = TmuxClient::local();
        alive = client.list_session_names().into_iter().collect();
    }
    let in_tmux = std::env::var_os("TMUX").is_some();
    let action = decide_tmux_attach_action(&target, &alive, print || plan_json, false, in_tmux);
    let session = match &action {
        TmuxAttachAction::Print { session }
        | TmuxAttachAction::SwitchClient { session }
        | TmuxAttachAction::Attach { session }
        | TmuxAttachAction::Recover { session } => session,
    };
    let stdout = if plan_json {
        render_attach_plan_json(&target, session, &action, readonly)
    } else {
        render_attach_plan_text(&target, session, &action, readonly)
    };
    let code = i32::from(matches!(action, TmuxAttachAction::Recover { .. }));
    CliOutput {
        code,
        stdout,
        stderr: String::new(),
    }
}

fn attach_usage_ok() -> CliOutput {
    CliOutput {
        code: 0,
        stdout: attach_usage_text(),
        stderr: String::new(),
    }
}

fn attach_usage_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\n{}", attach_usage_text()),
    }
}

fn attach_usage_text() -> String {
    "usage: maw-rs attach <target> [--print] [--readonly|-r]\n       maw-rs a <target> [--print] [--readonly|-r]\n".to_owned()
}

fn render_attach_plan_text(
    target: &str,
    session: &str,
    action: &TmuxAttachAction,
    readonly: bool,
) -> String {
    match action {
        TmuxAttachAction::Recover { .. } => format!(
            "attach: '{target}' resolved to missing session {session}\n  → maw wake {target} --attach\n"
        ),
        TmuxAttachAction::Print { .. }
        | TmuxAttachAction::SwitchClient { .. }
        | TmuxAttachAction::Attach { .. } => {
            let args = attach_command_args(action, readonly);
            format!(
                "Run: tmux {}\n  resolved: {target} → {session}\n  detach with: Ctrl-b d\n",
                args.join(" ")
            )
        }
    }
}

fn render_attach_plan_json(
    target: &str,
    session: &str,
    action: &TmuxAttachAction,
    readonly: bool,
) -> String {
    let kind = match action {
        TmuxAttachAction::Print { .. } => "print",
        TmuxAttachAction::SwitchClient { .. } => "switch-client",
        TmuxAttachAction::Attach { .. } => "attach",
        TmuxAttachAction::Recover { .. } => "recover",
    };
    let args = attach_command_args(action, readonly);
    format!(
        "{{\"command\":\"attach\",\"alias\":\"a\",\"target\":{},\"session\":{},\"action\":{},\"tmuxArgs\":{}}}\n",
        json_string(target),
        json_string(session),
        json_string(kind),
        json_string_array(&args)
    )
}

fn attach_command_args(action: &TmuxAttachAction, readonly: bool) -> Vec<String> {
    if readonly {
        let session = match action {
            TmuxAttachAction::Print { session }
            | TmuxAttachAction::SwitchClient { session }
            | TmuxAttachAction::Attach { session }
            | TmuxAttachAction::Recover { session } => session,
        };
        return vec!["attach".to_owned(), "-r".to_owned(), "-t".to_owned(), session.clone()];
    }
    tmux_attach_spawn_command(action).map_or_else(
        || {
            let session = match action {
                TmuxAttachAction::Print { session }
                | TmuxAttachAction::SwitchClient { session }
                | TmuxAttachAction::Attach { session }
                | TmuxAttachAction::Recover { session } => session,
            };
            vec!["attach".to_owned(), "-t".to_owned(), session.clone()]
        },
        |command| command.args,
    )
}
