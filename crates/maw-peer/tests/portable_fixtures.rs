use maw_peer::{
    parse_peer_source_mode, resolve_peer_sources, DiscoveryResult, DiscoveryRow, NamedPeerConfig,
    PeerConfig, PeerSourceKind, PeerSourceMode,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    mode: ModeFixture,
    config: ConfigFixture,
    discoveries: DiscoveriesFixture,
    expected: ExpectedFixture,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigFixture {
    #[serde(default)]
    peers: Vec<String>,
    #[serde(default)]
    named_peers: Vec<NamedPeerFixture>,
}

#[derive(Debug, Deserialize)]
struct NamedPeerFixture {
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedFixture {
    urls: Vec<String>,
    names: Vec<Option<String>>,
    sources: Vec<String>,
    warnings: Vec<String>,
    fetch_calls: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ModeFixture {
    Config,
    Scout,
    Both,
}

impl From<ModeFixture> for PeerSourceMode {
    fn from(value: ModeFixture) -> Self {
        match value {
            ModeFixture::Config => Self::Config,
            ModeFixture::Scout => Self::Scout,
            ModeFixture::Both => Self::Both,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DiscoveriesFixture {
    ok: bool,
    #[serde(default)]
    peers: Vec<DiscoveryRowFixture>,
    error: Option<String>,
    hint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscoveryRowFixture {
    node: Option<String>,
    oracle: Option<String>,
    host: Option<String>,
    locators: Vec<String>,
}

impl From<ConfigFixture> for PeerConfig {
    fn from(value: ConfigFixture) -> Self {
        Self {
            peers: value.peers,
            named_peers: value
                .named_peers
                .into_iter()
                .map(|peer| NamedPeerConfig {
                    name: peer.name,
                    url: peer.url,
                })
                .collect(),
        }
    }
}

impl From<DiscoveriesFixture> for DiscoveryResult {
    fn from(value: DiscoveriesFixture) -> Self {
        if value.ok {
            return Self::Ok {
                peers: value
                    .peers
                    .into_iter()
                    .map(|peer| DiscoveryRow {
                        node: peer.node,
                        oracle: peer.oracle,
                        host: peer.host,
                        locators: peer.locators,
                    })
                    .collect(),
            };
        }
        Self::Err {
            error: value.error.unwrap_or_default(),
            hint: value.hint,
        }
    }
}

fn source_name(source: PeerSourceKind) -> &'static str {
    match source {
        PeerSourceKind::Config => "config",
        PeerSourceKind::Scout => "scout",
    }
}

#[test]
fn peer_source_helpers_cover_mode_names_and_unavailable_scout_edges() {
    assert_eq!(PeerSourceMode::Config.as_str(), "config");
    assert_eq!(PeerSourceMode::Scout.as_str(), "scout");
    assert_eq!(PeerSourceMode::Both.as_str(), "both");
    assert_eq!(PeerSourceKind::Config.as_str(), "config");
    assert_eq!(PeerSourceKind::Scout.as_str(), "scout");

    assert_eq!(
        parse_peer_source_mode(None, PeerSourceMode::Both),
        Some(PeerSourceMode::Both)
    );
    assert_eq!(
        parse_peer_source_mode(Some(""), PeerSourceMode::Config),
        Some(PeerSourceMode::Config)
    );
    assert_eq!(
        parse_peer_source_mode(Some("config"), PeerSourceMode::Both),
        Some(PeerSourceMode::Config)
    );
    assert_eq!(
        parse_peer_source_mode(Some("scout"), PeerSourceMode::Both),
        Some(PeerSourceMode::Scout)
    );
    assert_eq!(
        parse_peer_source_mode(Some("both"), PeerSourceMode::Config),
        Some(PeerSourceMode::Both)
    );
    assert_eq!(
        parse_peer_source_mode(Some("invalid"), PeerSourceMode::Both),
        None
    );

    let missing = resolve_peer_sources(&PeerConfig::default(), PeerSourceMode::Scout, None);
    assert_eq!(missing.fetch_calls, 1);
    assert!(missing.peers.is_empty());
    assert_eq!(
        missing.warnings,
        vec!["scout unavailable (missing_discoveries)".to_owned()]
    );

    let no_hint = resolve_peer_sources(
        &PeerConfig::default(),
        PeerSourceMode::Both,
        Some(&DiscoveryResult::Err {
            error: "offline".to_owned(),
            hint: None,
        }),
    );
    assert_eq!(no_hint.warnings, vec!["scout unavailable (offline)"]);

    let host_fallback = resolve_peer_sources(
        &PeerConfig::default(),
        PeerSourceMode::Scout,
        Some(&DiscoveryResult::Ok {
            peers: vec![DiscoveryRow {
                node: None,
                oracle: Some("oracle".to_owned()),
                host: Some("host.local".to_owned()),
                locators: vec!["tcp://ignored".to_owned(), "HTTPS://host.local".to_owned()],
            }],
        }),
    );
    assert_eq!(host_fallback.peers[0].name.as_deref(), Some("host.local"));
    assert_eq!(host_fallback.peers[0].url, "HTTPS://host.local");
    assert_eq!(host_fallback.peers[0].oracle.as_deref(), Some("oracle"));
}

#[test]
fn peer_source_resolver_fixtures_match_maw_js_portable_spec() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("fixtures/peer-source-resolver.fixtures.json"))
            .expect("valid peer source resolver fixture json");

    for fixture in fixtures {
        let mode = PeerSourceMode::from(fixture.mode);
        let config = PeerConfig::from(fixture.config);
        let discoveries = DiscoveryResult::from(fixture.discoveries);
        let result = resolve_peer_sources(&config, mode, Some(&discoveries));

        assert_eq!(result.mode, mode, "mode: {}", fixture.name);
        assert_eq!(
            result
                .peers
                .iter()
                .map(|peer| peer.url.clone())
                .collect::<Vec<_>>(),
            fixture.expected.urls,
            "urls: {}",
            fixture.name
        );
        assert_eq!(
            result
                .peers
                .iter()
                .map(|peer| peer.name.clone())
                .collect::<Vec<_>>(),
            fixture.expected.names,
            "names: {}",
            fixture.name
        );
        assert_eq!(
            result
                .peers
                .iter()
                .map(|peer| source_name(peer.source).to_owned())
                .collect::<Vec<_>>(),
            fixture.expected.sources,
            "sources: {}",
            fixture.name
        );
        for warning in fixture.expected.warnings {
            assert!(
                result.warnings.join("\n").contains(&warning),
                "warning {warning:?}: {}",
                fixture.name
            );
        }
        assert_eq!(
            result.fetch_calls, fixture.expected.fetch_calls,
            "fetch calls: {}",
            fixture.name
        );
    }
}
