use maw_peer::{default_stale_ttl_ms, is_peer_stale, parse_stale_ttl_ms, stale_age_ms, PeerRecord};

fn peer(added_at: &str, last_seen: Option<&str>) -> PeerRecord {
    PeerRecord {
        url: "u".to_owned(),
        node: None,
        added_at: added_at.to_owned(),
        last_seen: last_seen.map(str::to_owned),
        last_error: None,
    }
}

#[test]
fn stale_ttl_parsing_matches_maw_js_env_contract() {
    assert_eq!(default_stale_ttl_ms(), 7 * 24 * 60 * 60 * 1000);
    assert_eq!(parse_stale_ttl_ms(None), default_stale_ttl_ms());
    assert_eq!(parse_stale_ttl_ms(Some("1234")), 1234);
    assert_eq!(parse_stale_ttl_ms(Some("0")), default_stale_ttl_ms());
    assert_eq!(parse_stale_ttl_ms(Some("-1")), default_stale_ttl_ms());
    assert_eq!(
        parse_stale_ttl_ms(Some("not-a-number")),
        default_stale_ttl_ms()
    );
    assert_eq!(parse_stale_ttl_ms(Some("")), default_stale_ttl_ms());
}

#[test]
fn stale_age_uses_last_seen_then_added_at_and_clamps_future_dates() {
    let now = 1_779_105_600_000; // 2026-05-18T12:00:00.000Z

    assert_eq!(
        stale_age_ms(&peer("2026-05-18T11:59:50.000Z", None), now),
        Some(10_000)
    );
    assert_eq!(
        stale_age_ms(
            &peer("2026-05-18T00:00:00.000Z", Some("2026-05-18T12:00:05.000Z")),
            now,
        ),
        Some(0)
    );
    assert_eq!(stale_age_ms(&peer("not-date", None), now), None);
}

#[test]
fn is_peer_stale_matches_maw_js_threshold_and_invalid_provenance_rules() {
    let now = 1_779_105_600_000;
    let ten_seconds_old = peer("2026-05-18T11:59:50.000Z", None);

    assert!(is_peer_stale(&ten_seconds_old, 9_999, now));
    assert!(!is_peer_stale(&ten_seconds_old, 10_000, now));
    assert!(is_peer_stale(&peer("not-date", None), 10_000, now));
}
