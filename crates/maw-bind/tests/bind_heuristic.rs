use maw_bind::{resolve_bind_host, BindConfig, BindHostReason};

const EMPTY: BindConfig = BindConfig {
    peers_len: 0,
    named_peers_len: 0,
};

#[test]
fn loopback_when_nothing_is_configured() {
    let result = resolve_bind_host(&EMPTY, None, Ok(0));
    assert_eq!(result.hostname, "127.0.0.1");
    assert_eq!(result.reason, None);
}

#[test]
fn trigger_1_config_peers_populated() {
    let result = resolve_bind_host(
        &BindConfig {
            peers_len: 1,
            named_peers_len: 0,
        },
        None,
        Ok(0),
    );
    assert_eq!(result.hostname, "0.0.0.0");
    assert_eq!(result.reason, Some(BindHostReason::ConfigPeers));
    assert_eq!(
        result.reason.map(BindHostReason::as_str),
        Some("config.peers")
    );
}

#[test]
fn trigger_2_config_named_peers_populated() {
    let result = resolve_bind_host(
        &BindConfig {
            peers_len: 0,
            named_peers_len: 1,
        },
        None,
        Ok(0),
    );
    assert_eq!(result.hostname, "0.0.0.0");
    assert_eq!(result.reason, Some(BindHostReason::ConfigNamedPeers));
}

#[test]
fn trigger_3_maw_host_zero_env_opt_in() {
    let result = resolve_bind_host(&EMPTY, Some("0.0.0.0"), Ok(0));
    assert_eq!(result.hostname, "0.0.0.0");
    assert_eq!(result.reason, Some(BindHostReason::MawHost));
}

#[test]
fn trigger_4_peers_json_non_empty() {
    let result = resolve_bind_host(&EMPTY, None, Ok(1));
    assert_eq!(result.hostname, "0.0.0.0");
    assert_eq!(result.reason, Some(BindHostReason::PeersJson));
}

#[test]
fn empty_peers_json_stays_on_loopback() {
    let result = resolve_bind_host(&EMPTY, None, Ok(0));
    assert_eq!(result.hostname, "127.0.0.1");
    assert_eq!(result.reason, None);
}

#[test]
fn maw_host_non_zero_value_does_not_trigger() {
    let result = resolve_bind_host(&EMPTY, Some("white"), Ok(0));
    assert_eq!(result.hostname, "127.0.0.1");
    assert_eq!(result.reason, None);
}

#[test]
fn empty_peers_arrays_do_not_trigger() {
    let result = resolve_bind_host(
        &BindConfig {
            peers_len: 0,
            named_peers_len: 0,
        },
        None,
        Ok(0),
    );
    assert_eq!(result.hostname, "127.0.0.1");
    assert_eq!(result.reason, None);
}

#[test]
fn peers_store_reader_error_falls_through_to_loopback() {
    let result = resolve_bind_host(&EMPTY, None, Err("disk read failed".to_owned()));
    assert_eq!(result.hostname, "127.0.0.1");
    assert_eq!(result.reason, None);
}

#[test]
fn config_peers_takes_priority_over_maw_host() {
    let result = resolve_bind_host(
        &BindConfig {
            peers_len: 1,
            named_peers_len: 0,
        },
        Some("0.0.0.0"),
        Ok(0),
    );
    assert_eq!(result.hostname, "0.0.0.0");
    assert_eq!(result.reason, Some(BindHostReason::ConfigPeers));
}
