//! Pure `maw bring` policy helpers ported from maw-js.
//!
//! This crate intentionally excludes tmux/runtime IO. Behavior is locked by
//! maw-js portable fixtures for `src/commands/shared/bring-flags.ts`.

/// Parsed `maw bring --to` destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BringToTarget {
    pub session: String,
    pub window: Option<String>,
}

/// Pure decision result for `maw bring --split` before tmux IO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitBringDecision {
    NoSplitRequested,
    Headless,
    RefuseSelfBring,
    RefuseSameSession,
    Split,
}

/// Inputs needed to decide whether a split-bring may proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitBringPolicy<'a> {
    pub split: bool,
    pub target: &'a str,
    pub caller_session_window: Option<&'a str>,
    pub split_target: Option<&'a str>,
    pub attached_to_tmux: bool,
    pub allow_self_bring: bool,
}

/// Translate `--to <session[:window]>` to wake-shaped flags.
///
/// `--to` without a following value is preserved so downstream parsing can
/// surface the same error class as maw-js.
#[must_use]
pub fn translate_bring_to_flag(argv: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(argv.len());
    let mut index = 0;
    while index < argv.len() {
        let arg = &argv[index];
        if arg == "--to" && index + 1 < argv.len() {
            index += 1;
            let target = parse_bring_to_target(&argv[index]);
            out.push("--session".to_owned());
            out.push(target.session.clone());
            if let Some(window) = target.window {
                out.push("--split-target".to_owned());
                out.push(format!("{}:{window}", target.session));
            }
        } else {
            out.push(arg.clone());
        }
        index += 1;
    }
    out
}

/// Parse a `--to` value that may contain a destination window.
#[must_use]
pub fn parse_bring_to_target(value: &str) -> BringToTarget {
    let Some((session, window)) = value.split_once(':') else {
        return BringToTarget {
            session: value.to_owned(),
            window: None,
        };
    };
    BringToTarget {
        session: session.to_owned(),
        window: (!window.is_empty()).then(|| window.to_owned()),
    }
}

/// Decide the `maybeSplit` guard path without executing tmux.
#[must_use]
pub fn decide_split_bring(policy: &SplitBringPolicy<'_>) -> SplitBringDecision {
    if !policy.split {
        return SplitBringDecision::NoSplitRequested;
    }
    if !policy.attached_to_tmux && policy.split_target.is_none() {
        return SplitBringDecision::Headless;
    }

    let caller_session_window = policy.caller_session_window;
    if !policy.allow_self_bring && is_self_bring(policy.target, caller_session_window) {
        return SplitBringDecision::RefuseSelfBring;
    }
    if same_session_target(policy.target, caller_session_window)
        && !(policy.allow_self_bring
            && caller_session_window.is_some()
            && is_self_bring(policy.target, caller_session_window))
    {
        return SplitBringDecision::RefuseSameSession;
    }

    SplitBringDecision::Split
}

/// Detect whether a split target points at the caller's own pane/window.
#[must_use]
pub fn is_self_bring(target: &str, caller_session_window: Option<&str>) -> bool {
    let Some(caller_session_window) = caller_session_window else {
        return false;
    };
    if target.is_empty() {
        return false;
    }

    let target_no_pane = strip_numeric_pane_suffix(target);
    if target_no_pane == caller_session_window {
        return true;
    }

    let caller_session = caller_session_window
        .split_once(':')
        .map_or(caller_session_window, |(session, _)| session);
    !target_no_pane.contains(':') && target_no_pane == caller_session
}

/// True when the target and caller are in the same tmux session.
#[must_use]
pub fn same_session_target(target: &str, caller_session_window: Option<&str>) -> bool {
    caller_session_window.is_some_and(|caller| target_session(target) == target_session(caller))
}

/// Return the tmux session component from a `session[:window[.pane]]` target.
#[must_use]
pub fn target_session(target: &str) -> &str {
    target
        .split_once(':')
        .map_or(target, |(session, _)| session)
}

fn strip_numeric_pane_suffix(value: &str) -> &str {
    let Some((head, suffix)) = value.rsplit_once('.') else {
        return value;
    };
    if !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        head
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_window_names_are_not_pane_suffixes() {
        assert_eq!(strip_numeric_pane_suffix("s:oracle.v2"), "s:oracle.v2");
        assert_eq!(strip_numeric_pane_suffix("s:oracle.12"), "s:oracle");
    }
}
