// Ported from maw-js test/isolated/should-auto-wake.test.ts and
// test/isolated/should-auto-wake-manifest.test.ts.

use maw_auto_wake::{should_auto_wake, AutoWakeManifest, AutoWakeOptions, AutoWakeSite};

fn decide(site: AutoWakeSite) -> maw_auto_wake::AutoWakeDecision {
    should_auto_wake(
        "neo",
        AutoWakeOptions {
            site,
            ..AutoWakeOptions::default()
        },
    )
}

fn manifest(sources: &[&str], is_live: bool) -> AutoWakeManifest {
    AutoWakeManifest {
        name: "neo".to_owned(),
        sources: sources.iter().map(|source| (*source).to_owned()).collect(),
        is_live,
    }
}

#[test]
fn fixed_contract_sites_match_maw_js_policy() {
    assert_eq!(decide(AutoWakeSite::Peek).reason, "peek never auto-wakes");
    assert!(!decide(AutoWakeSite::Peek).wake);
    assert_eq!(
        decide(AutoWakeSite::ApiWake).reason,
        "api-wake endpoint always wakes"
    );
    assert!(decide(AutoWakeSite::ApiWake).wake);
    assert_eq!(
        decide(AutoWakeSite::Bud).reason,
        "bud always wakes new oracle"
    );
    assert!(decide(AutoWakeSite::Bud).wake);

    assert!(
        !should_auto_wake(
            "x",
            AutoWakeOptions {
                site: AutoWakeSite::Peek,
                force: true,
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
    assert!(
        should_auto_wake(
            "x",
            AutoWakeOptions {
                site: AutoWakeSite::ApiWake,
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
    assert!(
        should_auto_wake(
            "x",
            AutoWakeOptions {
                site: AutoWakeSite::Bud,
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
}

#[test]
fn wake_cmd_is_idempotent_and_honors_operator_flags() {
    let missing = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::WakeCmd,
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(missing.wake);
    assert!(missing.reason.contains("missing"));

    let live = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::WakeCmd,
            is_live: Some(true),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!live.wake);
    assert!(live.reason.contains("already live"));

    assert_eq!(
        should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::WakeCmd,
                is_live: Some(false),
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--no-wake explicit deny"
    );
    assert_eq!(
        should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::WakeCmd,
                is_live: Some(true),
                force: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--wake explicit force"
    );
}

#[test]
fn view_policy_matches_fleet_known_prompt_and_flag_matrix() {
    let fleet_dead = should_auto_wake(
        "volt",
        AutoWakeOptions {
            site: AutoWakeSite::View,
            is_fleet_known: Some(true),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(fleet_dead.wake);
    assert!(fleet_dead.reason.contains("fleet-known"));

    let fleet_live = should_auto_wake(
        "volt",
        AutoWakeOptions {
            site: AutoWakeSite::View,
            is_fleet_known: Some(true),
            is_live: Some(true),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!fleet_live.wake);
    assert!(fleet_live.reason.contains("already running"));

    let unknown = should_auto_wake(
        "typo",
        AutoWakeOptions {
            site: AutoWakeSite::View,
            is_fleet_known: Some(false),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!unknown.wake);
    assert!(unknown.reason.contains("caller should ask"));

    assert_eq!(
        should_auto_wake(
            "volt",
            AutoWakeOptions {
                site: AutoWakeSite::View,
                is_fleet_known: Some(true),
                is_live: Some(false),
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--no-wake explicit deny"
    );
    assert_eq!(
        should_auto_wake(
            "typo",
            AutoWakeOptions {
                site: AutoWakeSite::View,
                is_fleet_known: Some(false),
                force: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--wake explicit force"
    );
    assert_eq!(
        should_auto_wake(
            "volt",
            AutoWakeOptions {
                site: AutoWakeSite::View,
                is_fleet_known: Some(true),
                force: true,
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--no-wake explicit deny"
    );
}

#[test]
fn hey_policy_matches_canonical_fleet_and_flag_matrix() {
    let fleet_dead = should_auto_wake(
        "volt",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_fleet_known: Some(true),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(fleet_dead.wake);
    assert!(fleet_dead.reason.contains("fleet-known"));

    assert!(
        !should_auto_wake(
            "volt",
            AutoWakeOptions {
                site: AutoWakeSite::Hey,
                is_fleet_known: Some(true),
                is_live: Some(true),
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );

    let canonical = should_auto_wake(
        "volt",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_fleet_known: Some(true),
            is_live: Some(false),
            is_canonical_target: true,
            ..AutoWakeOptions::default()
        },
    );
    assert!(!canonical.wake);
    assert!(canonical.reason.contains("canonical"));

    let unknown = should_auto_wake(
        "typo",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_fleet_known: Some(false),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!unknown.wake);
    assert!(unknown.reason.contains("unknown"));

    assert!(
        !should_auto_wake(
            "volt",
            AutoWakeOptions {
                site: AutoWakeSite::Hey,
                is_fleet_known: Some(true),
                is_live: Some(false),
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
    assert_eq!(
        should_auto_wake(
            "volt",
            AutoWakeOptions {
                site: AutoWakeSite::Hey,
                is_fleet_known: Some(true),
                is_live: Some(false),
                is_canonical_target: true,
                force: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--wake explicit force"
    );
}

#[test]
fn api_send_matches_fleet_known_and_live_policy() {
    let fleet_dead = should_auto_wake(
        "samba",
        AutoWakeOptions {
            site: AutoWakeSite::ApiSend,
            is_fleet_known: Some(true),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(fleet_dead.wake);
    assert!(fleet_dead.reason.contains("fleet-known"));

    assert!(
        !should_auto_wake(
            "samba",
            AutoWakeOptions {
                site: AutoWakeSite::ApiSend,
                is_fleet_known: Some(true),
                is_live: Some(true),
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );

    let unknown = should_auto_wake(
        "not-a-real-oracle",
        AutoWakeOptions {
            site: AutoWakeSite::ApiSend,
            is_fleet_known: Some(false),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!unknown.wake);
    assert!(unknown.reason.contains("unknown"));
}

#[test]
fn manifest_derives_fleet_known_and_live_and_wins_over_flags() {
    for site in [AutoWakeSite::View, AutoWakeSite::Hey, AutoWakeSite::ApiSend] {
        let decision = should_auto_wake(
            "neo",
            AutoWakeOptions {
                site,
                manifest: Some(manifest(&["fleet"], false)),
                ..AutoWakeOptions::default()
            },
        );
        assert!(decision.wake);
        assert!(decision.reason.contains("fleet-known"));
    }

    let no_fleet = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            manifest: Some(manifest(&["oracles-json"], false)),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!no_fleet.wake);
    assert!(no_fleet.reason.contains("unknown"));

    let view_no_fleet = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::View,
            manifest: Some(manifest(&["session", "agent"], false)),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!view_no_fleet.wake);
    assert!(view_no_fleet.reason.contains("caller should ask"));

    let live = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::View,
            is_live: Some(false),
            manifest: Some(manifest(&["fleet"], true)),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!live.wake);
    assert!(live.reason.contains("already running"));

    let manifest_wins_fleet = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_fleet_known: Some(false),
            is_live: Some(false),
            manifest: Some(manifest(&["fleet"], false)),
            ..AutoWakeOptions::default()
        },
    );
    assert!(manifest_wins_fleet.wake);

    let manifest_wins_unknown = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_fleet_known: Some(true),
            is_live: Some(false),
            manifest: Some(manifest(&["oracles-json"], false)),
            ..AutoWakeOptions::default()
        },
    );
    assert!(!manifest_wins_unknown.wake);
}

#[test]
fn manifest_still_allows_operator_and_fixed_contract_overrides() {
    assert_eq!(
        should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::Hey,
                manifest: Some(manifest(&["fleet"], false)),
                no_wake: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--no-wake explicit deny"
    );
    assert_eq!(
        should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::View,
                manifest: Some(manifest(&["oracles-json"], false)),
                force: true,
                ..AutoWakeOptions::default()
            }
        )
        .reason,
        "--wake explicit force"
    );

    let canonical = should_auto_wake(
        "neo",
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            manifest: Some(manifest(&["fleet"], false)),
            is_canonical_target: true,
            ..AutoWakeOptions::default()
        },
    );
    assert!(!canonical.wake);
    assert!(canonical.reason.contains("canonical"));

    assert!(
        !should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::Peek,
                manifest: Some(manifest(&["fleet"], false)),
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
    assert!(
        should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::ApiWake,
                manifest: Some(manifest(&["fleet"], true)),
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
    assert!(
        should_auto_wake(
            "neo",
            AutoWakeOptions {
                site: AutoWakeSite::Bud,
                manifest: Some(manifest(&["fleet"], true)),
                ..AutoWakeOptions::default()
            }
        )
        .wake
    );
}

#[test]
fn every_decision_returns_a_non_empty_reason() {
    let cases = [
        AutoWakeOptions {
            site: AutoWakeSite::Peek,
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::ApiWake,
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::Bud,
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::WakeCmd,
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::WakeCmd,
            is_live: Some(true),
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::View,
            is_fleet_known: Some(true),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::View,
            is_fleet_known: Some(false),
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_fleet_known: Some(true),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::Hey,
            is_canonical_target: true,
            ..AutoWakeOptions::default()
        },
        AutoWakeOptions {
            site: AutoWakeSite::ApiSend,
            is_fleet_known: Some(true),
            is_live: Some(false),
            ..AutoWakeOptions::default()
        },
    ];

    for opts in cases {
        let decision = should_auto_wake("x", opts);
        assert!(!decision.reason.is_empty());
    }
}
