//! Testable tmux command and parser adapter for maw-rs.
//!
//! This crate ports the deterministic parts of maw-js `src/core/transport/tmux-class.ts`:
//! shell-safe command construction plus parsing of `list-windows` / `list-panes` output.
//! Real process execution is intentionally injected through [`TmuxRunner`].

use std::{collections::BTreeSet, error::Error, fmt};

/// Tmux window metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxWindow {
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub cwd: Option<String>,
}

/// Tmux session metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxSession {
    pub name: String,
    pub windows: Vec<TmuxWindow>,
}

/// Tmux pane metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxPane {
    pub id: String,
    pub command: String,
    pub target: String,
    pub title: String,
    pub pid: Option<u32>,
    pub cwd: Option<String>,
    pub last_activity: Option<u64>,
}

/// Error returned by an injected tmux runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxError {
    pub message: String,
}

impl TmuxError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TmuxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for TmuxError {}

/// Injectable tmux execution seam.
pub trait TmuxRunner {
    /// Run `tmux <subcommand> <args...>` and return stdout.
    ///
    /// # Errors
    ///
    /// Returns [`TmuxError`] when tmux exits non-zero or the host command cannot be executed.
    fn run(&mut self, subcommand: &str, args: &[String]) -> Result<String, TmuxError>;
}

/// Testable tmux client that delegates all execution to [`TmuxRunner`].
pub struct TmuxClient<R> {
    runner: R,
}

impl<R> TmuxClient<R>
where
    R: TmuxRunner,
{
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }

    /// List session names; tmux-unavailable errors are fail-soft and return an empty list.
    pub fn list_session_names(&mut self) -> Vec<String> {
        self.runner
            .run(
                "list-sessions",
                &["-F".to_owned(), "#{session_name}".to_owned()],
            )
            .map(|raw| parse_session_names(&raw))
            .unwrap_or_default()
    }

    /// List all sessions/windows in a single tmux call; tmux-unavailable errors return empty.
    pub fn list_all(&mut self) -> Vec<TmuxSession> {
        self.runner
            .run(
                "list-windows",
                &[
                    "-a".to_owned(),
                    "-F".to_owned(),
                    "#{session_name}|||#{window_index}|||#{window_name}|||#{window_active}|||#{pane_current_path}".to_owned(),
                ],
            )
            .map(|raw| parse_list_all_windows(&raw))
            .unwrap_or_default()
    }

    /// List one session's windows.
    ///
    /// # Errors
    ///
    /// Returns the injected runner error when tmux rejects the session target.
    pub fn list_windows(&mut self, session: &str) -> Result<Vec<TmuxWindow>, TmuxError> {
        let raw = self.runner.run(
            "list-windows",
            &[
                "-t".to_owned(),
                session.to_owned(),
                "-F".to_owned(),
                "#{window_index}:#{window_name}:#{window_active}".to_owned(),
            ],
        )?;
        Ok(parse_list_windows(&raw))
    }

    /// Get all pane IDs; tmux-unavailable errors return empty.
    pub fn list_pane_ids(&mut self) -> BTreeSet<String> {
        self.runner
            .run(
                "list-panes",
                &["-a".to_owned(), "-F".to_owned(), "#{pane_id}".to_owned()],
            )
            .map(|raw| parse_pane_ids(&raw))
            .unwrap_or_default()
    }

    /// Get structured pane information; tmux-unavailable errors return empty.
    pub fn list_panes(&mut self) -> Vec<TmuxPane> {
        self.runner
            .run(
                "list-panes",
                &[
                    "-a".to_owned(),
                    "-F".to_owned(),
                    "#{pane_id}|||#{pane_current_command}|||#{session_name}:#{window_name}.#{pane_index}|||#{pane_title}|||#{pane_pid}|||#{pane_current_path}|||#{window_activity}".to_owned(),
                ],
            )
            .map(|raw| parse_list_panes(&raw))
            .unwrap_or_default()
    }
}

/// Shell-quote one tmux command argument using the same safe-character policy as maw-js.
#[must_use]
pub fn shell_quote(value: impl fmt::Display) -> String {
    let value = value.to_string();
    if !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-' | b'/')
        })
    {
        value
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

/// Build the shell command used by maw-js-style `tmux [-S socket] subcommand args...` execution.
#[must_use]
pub fn tmux_shell_command(socket: Option<&str>, subcommand: &str, args: &[String]) -> String {
    let socket_flag =
        socket.map_or_else(String::new, |socket| format!("-S {} ", shell_quote(socket)));
    let joined_args = args.iter().map(shell_quote).collect::<Vec<_>>().join(" ");
    if joined_args.is_empty() {
        format!("tmux {socket_flag}{subcommand}")
    } else {
        format!("tmux {socket_flag}{subcommand} {joined_args}")
    }
}

/// Parse `tmux list-sessions -F '#{session_name}'` output.
#[must_use]
pub fn parse_session_names(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Parse maw-js `list-windows -a` format.
#[must_use]
pub fn parse_list_all_windows(raw: &str) -> Vec<TmuxSession> {
    let mut sessions: Vec<TmuxSession> = Vec::new();
    for line in raw.lines().filter(|line| !line.is_empty()) {
        let fields = line.split("|||").collect::<Vec<_>>();
        if fields.len() < 4 {
            continue;
        }
        let session_name = fields[0];
        let window = TmuxWindow {
            index: fields[1].parse().unwrap_or(0),
            name: fields[2].to_owned(),
            active: fields[3] == "1",
            cwd: fields
                .get(4)
                .and_then(|cwd| (!cwd.is_empty()).then(|| (*cwd).to_owned())),
        };
        if let Some(session) = sessions
            .iter_mut()
            .find(|session| session.name == session_name)
        {
            session.windows.push(window);
        } else {
            sessions.push(TmuxSession {
                name: session_name.to_owned(),
                windows: vec![window],
            });
        }
    }
    sessions
}

/// Parse maw-js `list-windows -t <session> -F '#{window_index}:#{window_name}:#{window_active}'` output.
#[must_use]
pub fn parse_list_windows(raw: &str) -> Vec<TmuxWindow> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut parts = line.splitn(3, ':');
            let index = parts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let name = parts.next().unwrap_or_default().to_owned();
            let active = parts.next() == Some("1");
            TmuxWindow {
                index,
                name,
                active,
                cwd: None,
            }
        })
        .collect()
}

/// Parse `tmux list-panes -a -F '#{pane_id}'` output.
#[must_use]
pub fn parse_pane_ids(raw: &str) -> BTreeSet<String> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Parse maw-js structured `list-panes -a` format.
#[must_use]
pub fn parse_list_panes(raw: &str) -> Vec<TmuxPane> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let fields = line.split("|||").collect::<Vec<_>>();
            (fields.len() >= 4).then(|| TmuxPane {
                id: fields[0].to_owned(),
                command: fields[1].to_owned(),
                target: fields[2].to_owned(),
                title: fields[3].to_owned(),
                pid: fields.get(4).and_then(|pid| pid.parse().ok()),
                cwd: fields
                    .get(5)
                    .and_then(|cwd| (!cwd.is_empty()).then(|| (*cwd).to_owned())),
                last_activity: fields.get(6).and_then(|activity| activity.parse().ok()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeRunner {
        calls: Vec<(String, Vec<String>)>,
        responses: Vec<Result<String, TmuxError>>,
    }

    impl FakeRunner {
        fn with_responses(responses: Vec<Result<&str, TmuxError>>) -> Self {
            Self {
                calls: Vec::new(),
                responses: responses
                    .into_iter()
                    .map(|response| response.map(str::to_owned))
                    .collect(),
            }
        }
    }

    impl TmuxRunner for FakeRunner {
        fn run(&mut self, subcommand: &str, args: &[String]) -> Result<String, TmuxError> {
            self.calls.push((subcommand.to_owned(), args.to_vec()));
            if self.responses.is_empty() {
                return Err(TmuxError::new("no response"));
            }
            self.responses.remove(0)
        }
    }

    #[test]
    fn shell_quote_matches_maw_js_safe_chars_and_single_quote_escape() {
        assert_eq!(
            shell_quote("alpha_1:/tmp/repo.wt-main"),
            "alpha_1:/tmp/repo.wt-main"
        );
        assert_eq!(shell_quote("two words"), "'two words'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn tmux_shell_command_includes_optional_socket() {
        assert_eq!(
            tmux_shell_command(
                Some("/tmp/maw sock"),
                "list-windows",
                &[
                    "-a".to_owned(),
                    "-F".to_owned(),
                    "#{window_name}".to_owned()
                ],
            ),
            "tmux -S '/tmp/maw sock' list-windows -a -F '#{window_name}'",
        );
    }

    #[test]
    fn parse_list_all_groups_windows_by_session_in_order() {
        let sessions = parse_list_all_windows(
            "s1|||1|||alpha|||1|||/tmp/a\ns1|||2|||beta|||0|||\ns2|||1|||gamma|||0|||/tmp/g\n",
        );
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "s1");
        assert_eq!(sessions[0].windows[0].cwd.as_deref(), Some("/tmp/a"));
        assert_eq!(sessions[0].windows[1].cwd, None);
        assert!(sessions[0].windows[0].active);
        assert_eq!(sessions[1].windows[0].name, "gamma");
    }

    #[test]
    fn parse_list_windows_matches_maw_js_colon_format() {
        assert_eq!(
            parse_list_windows("1:oracle:1\n2:notes:0\n"),
            vec![
                TmuxWindow {
                    index: 1,
                    name: "oracle".to_owned(),
                    active: true,
                    cwd: None
                },
                TmuxWindow {
                    index: 2,
                    name: "notes".to_owned(),
                    active: false,
                    cwd: None
                },
            ],
        );
    }

    #[test]
    fn parse_list_panes_handles_optional_numeric_fields() {
        let panes = parse_list_panes(
            "%1|||claude|||s:oracle.0|||title|||123|||/repo|||456\n%2|||zsh|||s:logs.0|||||||||\n",
        );
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].pid, Some(123));
        assert_eq!(panes[0].cwd.as_deref(), Some("/repo"));
        assert_eq!(panes[0].last_activity, Some(456));
        assert_eq!(panes[1].pid, None);
    }

    #[test]
    fn client_fail_soft_lists_and_records_runner_args() {
        let runner =
            FakeRunner::with_responses(vec![Ok("s1\ns2\n"), Err(TmuxError::new("no server"))]);
        let mut client = TmuxClient::new(runner);
        assert_eq!(client.list_session_names(), vec!["s1", "s2"]);
        assert!(client.list_all().is_empty());
        assert_eq!(client.runner.calls[0].0, "list-sessions");
        assert_eq!(client.runner.calls[1].0, "list-windows");
    }
}
