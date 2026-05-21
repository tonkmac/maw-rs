use std::{
    io::IsTerminal,
    process::{Command, Stdio},
};

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
    if !std::io::stdout().is_terminal() {
        return None;
    }

    let mut readonly = false;
    let mut target: Option<&str> = None;
    let mut index = 1;
    while index < argv.len() {
        match argv[index].as_str() {
            "--readonly" | "--read-only" | "-r" => readonly = true,
            arg if arg.starts_with('-') => return None,
            value => {
                if target.is_some() {
                    return None;
                }
                target = Some(value);
            }
        }
        index += 1;
    }
    let target = target?;
    let session = target.split(':').next().unwrap_or(target);
    let mut tmux_args = Vec::new();
    if readonly {
        tmux_args.extend(["attach", "-r", "-t", session]);
    } else if std::env::var_os("TMUX").is_some() {
        tmux_args.extend(["switch-client", "-t", session]);
    } else {
        tmux_args.extend(["attach", "-t", session]);
    }
    let status = Command::new("tmux")
        .args(tmux_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    Some(match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("attach: failed to execute tmux: {error}");
            1
        }
    })
}
