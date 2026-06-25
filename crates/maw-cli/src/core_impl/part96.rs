const DISPATCH_96: &[DispatcherEntry] = &[DispatcherEntry {
    command: "auth",
    handler: Handler::Sync(auth_run_command),
}];

const AUTH_VALUE_FLAGS: &[&str] = &[
    "--token",
    "--method",
    "--path",
    "--now",
    "--body-hash",
    "--signed-at",
    "--signature",
    "--cached-pubkey",
    "--from",
    "--timestamp",
    "--signature-v3",
    "--peer-key",
    "--address",
    "--oracle",
    "--node",
    "--body",
    "--secret",
    "--payload",
    "--header",
];

const AUTH_CONSENT_MUTATING_SUBCOMMANDS: &[&str] = &[
    "approve",
    "auto-approve",
    "pair-approve",
    "pair-auto",
    "trust",
];

fn auth_run_command(argv: &[String]) -> CliOutput {
    match auth_validate_argv(argv) {
        Ok(()) => run_auth_plan(argv),
        Err(message) => auth_guard_error(&message),
    }
}

fn auth_validate_argv(argv: &[String]) -> Result<(), String> {
    auth_validate_subcommand(argv)?;
    auth_validate_separator(argv)?;
    auth_validate_leading_dash_values(argv)?;
    Ok(())
}

fn auth_validate_subcommand(argv: &[String]) -> Result<(), String> {
    let Some(kind) = argv.first() else { return Ok(()); };
    if kind == "--" || kind.starts_with('-') {
        return Err("auth subcommand must not start with '-'".to_owned());
    }
    if AUTH_CONSENT_MUTATING_SUBCOMMANDS.iter().any(|blocked| blocked == kind) {
        return Err("auth: consent mutation requires explicit human flow; no auto-approve surface is exposed".to_owned());
    }
    Ok(())
}

fn auth_validate_separator(argv: &[String]) -> Result<(), String> {
    if argv.iter().any(|arg| arg == "--") {
        return Err("auth: -- separator is not allowed".to_owned());
    }
    Ok(())
}

fn auth_validate_leading_dash_values(argv: &[String]) -> Result<(), String> {
    let mut index = 1_usize;
    while index < argv.len() {
        let arg = &argv[index];
        if auth_is_value_flag(arg) {
            auth_validate_flag_value(argv, index, arg)?;
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn auth_is_value_flag(arg: &str) -> bool {
    AUTH_VALUE_FLAGS.iter().any(|flag| flag == &arg)
}

fn auth_validate_flag_value(argv: &[String], index: usize, flag: &str) -> Result<(), String> {
    let Some(value) = argv.get(index + 1) else { return Ok(()); };
    if value == "--" || value.starts_with('-') {
        return Err(format!("auth: {flag} value must not start with '-'"));
    }
    auth_validate_control_free_value(flag, value)
}

fn auth_validate_control_free_value(flag: &str, value: &str) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!("auth: {flag} value must not contain control characters"));
    }
    Ok(())
}

fn auth_guard_error(message: &str) -> CliOutput {
    CliOutput {
        code: 2,
        stdout: String::new(),
        stderr: format!("{message}\n"),
    }
}

#[cfg(test)]
mod auth_native_tests {
    use super::*;

    fn auth_args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn auth_dispatch_registers_single_native_command() {
        assert_eq!(DISPATCH_96.len(), 1);
        assert_eq!(DISPATCH_96[0].command, "auth");
        assert_eq!(dispatcher_status("auth"), DispatchKind::Native);
    }

    #[test]
    fn auth_sign_uses_fake_token_without_echoing_secret() {
        let output = auth_run_command(&auth_args(&[
            "sign-v1",
            "--token",
            "fake-test-token",
            "--now",
            "123",
            "--plan-json",
        ]));
        assert_eq!(output.code, 0, "{}", output.stderr);
        assert!(output.stdout.contains("\"kind\":\"sign-v1\""));
        assert!(!output.stdout.contains("fake-test-token"));
        assert!(!output.stderr.contains("fake-test-token"));
    }

    #[test]
    fn auth_guards_separator_and_leading_dash_values_without_secret_echo() {
        let sep = auth_run_command(&auth_args(&["sign-v1", "--"]));
        assert_eq!(sep.code, 2);
        assert!(sep.stderr.contains("separator"));
        let guarded = auth_run_command(&auth_args(&["sign-v1", "--token", "-secret-token"]));
        assert_eq!(guarded.code, 2);
        assert!(guarded.stderr.contains("--token value must not start"));
        assert!(!guarded.stderr.contains("secret-token"));
    }

    #[test]
    fn auth_blocks_auto_approve_consent_surface() {
        let output = auth_run_command(&auth_args(&["auto-approve", "--token", "fake-test-token"]));
        assert_eq!(output.code, 2);
        assert!(output.stderr.contains("no auto-approve"));
        assert!(!output.stderr.contains("fake-test-token"));
    }

    #[test]
    fn auth_verify_request_header_remains_plan_only_and_guarded() {
        let output = auth_run_command(&auth_args(&[
            "verify-request",
            "--now",
            "123",
            "--header",
            "x-maw-from=mawjs:m5",
            "--plan-json",
        ]));
        assert_eq!(output.code, 0, "{}", output.stderr);
        assert!(output.stdout.contains("\"command\":\"auth\""));
        let bad = auth_run_command(&auth_args(&["verify-request", "--header", "-bad"]));
        assert_eq!(bad.code, 2);
        assert!(bad.stderr.contains("--header value must not start"));
    }
}
