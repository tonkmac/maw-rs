const DISPATCH_50: &[DispatcherEntry] = &[DispatcherEntry {
    command: "oracle-skills",
    handler: Handler::Sync(oracle_skills_run_command),
}];

fn oracle_skills_run_command(argv: &[String]) -> CliOutput {
    oracle_skills_with_runner(argv, oracle_skills_run_process)
}

fn oracle_skills_with_runner(
    argv: &[String],
    runner: impl FnOnce(&[String]) -> Result<std::process::Output, std::io::Error>,
) -> CliOutput {
    match runner(argv) {
        Ok(output) if output.status.success() => CliOutput {
            code: 0,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Ok(output) => {
            let code = output.status.code().unwrap_or(1);
            let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
            stderr.push_str("arra-oracle-skills exited with code ");
            stderr.push_str(&code.to_string());
            stderr.push('\n');
            CliOutput {
                code,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr,
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: "arra-oracle-skills not found on $PATH. Install with: bun add -g arra-oracle-skills\n".to_owned(),
        },
        Err(error) => CliOutput {
            code: 1,
            stdout: String::new(),
            stderr: format!("failed to execute arra-oracle-skills: {error}\n"),
        },
    }
}

fn oracle_skills_run_process(argv: &[String]) -> Result<std::process::Output, std::io::Error> {
    std::process::Command::new("arra-oracle-skills")
        .args(argv)
        .output()
}

#[cfg(test)]
mod oracle_skills_tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn oracle_skills_status(code: i32) -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(code << 8)
    }

    fn oracle_skills_output(code: i32, stdout: &str, stderr: &str) -> std::process::Output {
        std::process::Output {
            status: oracle_skills_status(code),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn oracle_skills_runner_preserves_stdout_stderr_and_exit_code() {
        let output = oracle_skills_with_runner(&["--help".to_owned()], |_| {
            Ok(oracle_skills_output(7, "child out\n", "child err\n"))
        });

        assert_eq!(output.code, 7);
        assert_eq!(output.stdout, "child out\n");
        assert_eq!(
            output.stderr,
            "child err\narra-oracle-skills exited with code 7\n"
        );
    }

    #[test]
    fn oracle_skills_missing_binary_matches_maw_js_install_hint() {
        let output = oracle_skills_with_runner(&[], |_| {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"))
        });

        assert_eq!(output.code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(
            output.stderr,
            "arra-oracle-skills not found on $PATH. Install with: bun add -g arra-oracle-skills\n"
        );
    }
}
