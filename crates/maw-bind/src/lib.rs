//! Bind-host heuristic ported from maw-js `src/core/bind-host.ts`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindHostReason {
    ConfigPeers,
    ConfigNamedPeers,
    MawHost,
    PeersJson,
}

impl BindHostReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfigPeers => "config.peers",
            Self::ConfigNamedPeers => "config.namedPeers",
            Self::MawHost => "MAW_HOST",
            Self::PeersJson => "peers.json",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindConfig {
    pub peers_len: usize,
    pub named_peers_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindHostResult {
    pub hostname: String,
    pub reason: Option<BindHostReason>,
}

#[must_use]
pub fn resolve_bind_host(
    config: &BindConfig,
    maw_host: Option<&str>,
    peers_store_len: Result<usize, String>,
) -> BindHostResult {
    if config.peers_len > 0 {
        return exposed(BindHostReason::ConfigPeers);
    }
    if config.named_peers_len > 0 {
        return exposed(BindHostReason::ConfigNamedPeers);
    }
    if maw_host == Some("0.0.0.0") {
        return exposed(BindHostReason::MawHost);
    }
    if peers_store_len.is_ok_and(|len| len > 0) {
        return exposed(BindHostReason::PeersJson);
    }
    BindHostResult {
        hostname: "127.0.0.1".to_owned(),
        reason: None,
    }
}

fn exposed(reason: BindHostReason) -> BindHostResult {
    BindHostResult {
        hostname: "0.0.0.0".to_owned(),
        reason: Some(reason),
    }
}
