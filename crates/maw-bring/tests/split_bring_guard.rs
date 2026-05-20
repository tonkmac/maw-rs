use maw_bring::{decide_split_bring, same_session_target, SplitBringDecision, SplitBringPolicy};

#[test]
fn split_bring_refuses_self_target_unless_explicitly_overridden() {
    // Ported from maw-js test/wake-maybe-split-coverage.test.ts:
    // #1816 self-bring guard and MAW_ALLOW_SELF_BRING override.
    let policy = SplitBringPolicy {
        split: true,
        target: "20-homekeeper:homekeeper-oracle",
        caller_session_window: Some("20-homekeeper:homekeeper-oracle"),
        split_target: None,
        attached_to_tmux: true,
        allow_self_bring: false,
    };
    assert_eq!(
        decide_split_bring(&policy),
        SplitBringDecision::RefuseSelfBring
    );

    let allowed = SplitBringPolicy {
        allow_self_bring: true,
        ..policy
    };
    assert_eq!(decide_split_bring(&allowed), SplitBringDecision::Split);
}

#[test]
fn split_bring_refuses_different_window_in_same_session() {
    // Ported from maw-js #1835 coverage: nested same-session attach would
    // close/smear the caller pane, so the split path must stop before tmux IO.
    let policy = SplitBringPolicy {
        split: true,
        target: "20-homekeeper:homekeeper-bridge",
        caller_session_window: Some("20-homekeeper:homekeeper-oracle"),
        split_target: None,
        attached_to_tmux: true,
        allow_self_bring: false,
    };

    assert_eq!(
        decide_split_bring(&policy),
        SplitBringDecision::RefuseSameSession
    );
}

#[test]
fn split_bring_refuses_explicit_to_anchor_inside_same_session() {
    // Ported from maw-js #1827/#1836 coverage: explicit --to in the target
    // session is still a same-session nested attach risk.
    let policy = SplitBringPolicy {
        split: true,
        target: "50-mawjs:mawjs-features",
        caller_session_window: Some("50-mawjs:mawjs-oracle"),
        split_target: Some("50-mawjs:mawjs-oracle"),
        attached_to_tmux: true,
        allow_self_bring: false,
    };

    assert_eq!(
        decide_split_bring(&policy),
        SplitBringDecision::RefuseSameSession
    );
}

#[test]
fn split_bring_allows_cross_session_targets_from_claude_like_panes_by_default() {
    // Ported from maw-js #1836 coverage: Claude-like callers split
    // cross-session targets by default; only same-session targets are refused.
    let policy = SplitBringPolicy {
        split: true,
        target: "20-homekeeper:homekeeper-oracle",
        caller_session_window: Some("50-mawjs:mawjs-oracle"),
        split_target: None,
        attached_to_tmux: true,
        allow_self_bring: false,
    };

    assert_eq!(decide_split_bring(&policy), SplitBringDecision::Split);
}

#[test]
fn split_bring_reports_noop_and_headless_paths_before_tmux_io() {
    let no_split = SplitBringPolicy {
        split: false,
        target: "20-homekeeper:homekeeper-oracle",
        caller_session_window: Some("50-mawjs:mawjs-oracle"),
        split_target: None,
        attached_to_tmux: true,
        allow_self_bring: false,
    };
    assert_eq!(
        decide_split_bring(&no_split),
        SplitBringDecision::NoSplitRequested
    );

    let headless = SplitBringPolicy {
        split: true,
        attached_to_tmux: false,
        caller_session_window: None,
        ..no_split
    };
    assert_eq!(decide_split_bring(&headless), SplitBringDecision::Headless);
}

#[test]
fn same_session_target_matches_only_session_component() {
    assert!(same_session_target(
        "50-mawjs:mawjs-features",
        Some("50-mawjs:mawjs-oracle")
    ));
    assert!(same_session_target(
        "50-mawjs",
        Some("50-mawjs:mawjs-oracle")
    ));
    assert!(!same_session_target(
        "20-homekeeper:homekeeper-oracle",
        Some("50-mawjs:mawjs-oracle")
    ));
    assert!(!same_session_target("50-mawjs", None));
}
