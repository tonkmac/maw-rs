//! Portable transport classification and failover routing.
//!
//! This crate mirrors the pure send-order behavior in maw-js
//! `src/core/transport/transport.ts` without binding to async runtime or IO.

/// Transport failure reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportFailureReason {
    Timeout,
    Unreachable,
    Auth,
    RateLimit,
    Rejected,
    ParseError,
    Unknown,
}

impl TransportFailureReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Unreachable => "unreachable",
            Self::Auth => "auth",
            Self::RateLimit => "rate_limit",
            Self::Rejected => "rejected",
            Self::ParseError => "parse_error",
            Self::Unknown => "unknown",
        }
    }
}

/// Classified transport failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassifiedError {
    pub reason: TransportFailureReason,
    pub retryable: bool,
}

/// Classify common error strings into portable failure reasons.
#[must_use]
pub fn classify_error(err: Option<&str>) -> ClassifiedError {
    let Some(err) = err else {
        return ClassifiedError {
            reason: TransportFailureReason::Unknown,
            retryable: false,
        };
    };
    let msg = err.to_lowercase();
    if contains_any(&msg, &["timeout", "etimedout", "econnreset"]) {
        return ClassifiedError {
            reason: TransportFailureReason::Timeout,
            retryable: true,
        };
    }
    if contains_any(&msg, &["econnrefused", "unreachable", "enetunreach"]) {
        return ClassifiedError {
            reason: TransportFailureReason::Unreachable,
            retryable: true,
        };
    }
    if contains_any(&msg, &["401", "403", "auth", "unauthorized", "forbidden"]) {
        return ClassifiedError {
            reason: TransportFailureReason::Auth,
            retryable: false,
        };
    }
    if msg.contains("429") || msg.contains("too many") || rate_limit_like(&msg) {
        return ClassifiedError {
            reason: TransportFailureReason::RateLimit,
            retryable: true,
        };
    }
    if contains_any(&msg, &["400", "reject", "denied"]) {
        return ClassifiedError {
            reason: TransportFailureReason::Rejected,
            retryable: false,
        };
    }
    if contains_any(&msg, &["parse", "json", "syntax"]) {
        return ClassifiedError {
            reason: TransportFailureReason::ParseError,
            retryable: false,
        };
    }
    ClassifiedError {
        reason: TransportFailureReason::Unknown,
        retryable: false,
    }
}

/// Result of a routed send attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportResult {
    pub ok: bool,
    pub via: String,
    pub reason: Option<TransportFailureReason>,
    pub retryable: bool,
}

impl TransportResult {
    #[must_use]
    pub fn success(via: impl Into<String>) -> Self {
        Self {
            ok: true,
            via: via.into(),
            reason: None,
            retryable: false,
        }
    }

    #[must_use]
    pub fn failure(
        via: impl Into<String>,
        reason: TransportFailureReason,
        retryable: bool,
    ) -> Self {
        Self {
            ok: false,
            via: via.into(),
            reason: Some(reason),
            retryable,
        }
    }
}

/// Destination metadata for transport selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportTarget {
    pub oracle: String,
    pub host: Option<String>,
    pub tmux_target: Option<String>,
}

/// Window shape used by local tmux target resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxTransportWindow {
    pub index: u32,
    pub name: String,
    pub active: bool,
}

/// Session shape used by local tmux target resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxTransportSession {
    pub name: String,
    pub windows: Vec<TmuxTransportWindow>,
}

/// HTTP federation transport configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpTransportConfig {
    pub peers: Vec<String>,
    pub self_host: String,
}

/// Result of an HTTP feed publish attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpPostResult {
    pub ok: bool,
    pub status: u16,
}

/// Captured warning for failed best-effort HTTP feed publishing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpFeedWarning {
    pub peer: String,
    pub reason: String,
}

/// Locally measured federation status: local URL plus one-way reachability to peers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationStatus {
    pub local_url: String,
    pub peers: Vec<FederationPeerStatus>,
}

/// One peer row from the local federation status baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationPeerStatus {
    pub url: String,
    pub node: Option<String>,
    pub reachable: bool,
    pub latency: Option<u64>,
    pub agents: Vec<String>,
    pub clock_warning: bool,
}

/// One peer row reported by a remote peer's federation status endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationPeerView {
    pub url: Option<String>,
    pub node: Option<String>,
    pub reachable: Option<bool>,
}

/// Remote `/api/federation/status` result supplied by the IO adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerFederationStatusResult {
    Ok(PeerFederationStatus),
    MissingPeers,
    HttpStatus(u16),
    FetchError(String),
}

/// Decoded remote federation status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerFederationStatus {
    pub peers: Vec<FederationPeerView>,
}

/// Symmetric pair-health classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairHealth {
    Healthy,
    HalfUp,
    Down,
    Unknown,
}

impl PairHealth {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::HalfUp => "half-up",
            Self::Healthy => "healthy",
            Self::Unknown => "unknown",
        }
    }
}

/// Pair-health row for a single local peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairStatus {
    pub url: String,
    pub node: Option<String>,
    pub pair: PairHealth,
    pub forward: bool,
    pub reverse: Option<bool>,
    pub reason: Option<String>,
    pub latency: Option<u64>,
    pub agents: Vec<String>,
    pub clock_warning: bool,
}

/// Complete symmetric federation status summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymmetricFederationStatus {
    pub local_url: String,
    pub local_node: String,
    pub pairs: Vec<PairStatus>,
    pub healthy_pairs: usize,
    pub total_pairs: usize,
}

/// Side-effect seam for HTTP federation transport.
pub trait HttpTransportIo {
    /// List local sessions before aggregating remote peer sessions.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific error string when local listing fails.
    fn list_local_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String>;

    /// Return local + remote sessions, preserving any source metadata.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific error string when aggregation fails.
    fn get_all_sessions(
        &mut self,
        local_sessions: &[TmuxTransportSession],
    ) -> Result<Vec<TransportSession>, String>;

    /// Resolve a window in a single remote session.
    fn find_target_window(&mut self, sessions: &[TransportSession], query: &str) -> Option<String>;

    /// Send keys to a remote peer/source.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific error string when peer send fails.
    fn send_peer_keys(&mut self, source: &str, target: &str, message: &str)
        -> Result<bool, String>;

    /// POST a feed event to a peer.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific error string when publishing fails.
    fn post_peer_feed(
        &mut self,
        url: &str,
        method: &str,
        body: &str,
        timeout_ms: u64,
    ) -> Result<HttpPostResult, String>;

    /// Return configured timeout for a named transport.
    fn timeout_for(&self, transport: &str) -> u64;
}

/// Session shape used by HTTP federation, including source peer metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportSession {
    pub name: String,
    pub source: Option<String>,
    pub windows: Vec<TmuxTransportWindow>,
}

impl From<TmuxTransportSession> for TransportSession {
    fn from(value: TmuxTransportSession) -> Self {
        Self {
            name: value.name,
            source: None,
            windows: value.windows,
        }
    }
}

/// Minimal portable transport trait.
pub trait Transport {
    fn name(&self) -> &str;
    fn connected(&self) -> bool;
    fn can_reach(&self, target: &TransportTarget) -> bool;
    /// Send a message through this transport.
    ///
    /// # Errors
    ///
    /// Returns an error string when the transport attempted delivery but failed.
    /// The router classifies that error to decide whether to fail over.
    fn send(&mut self, target: &TransportTarget, message: &str, from: &str)
        -> Result<bool, String>;
}

/// Ordered transport router. First successful reachable transport wins.
#[derive(Default)]
pub struct TransportRouter<T> {
    transports: Vec<T>,
}

impl<T> TransportRouter<T>
where
    T: Transport,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            transports: Vec::new(),
        }
    }

    pub fn register(&mut self, transport: T) {
        self.transports.push(transport);
    }

    pub fn send(&mut self, target: &TransportTarget, message: &str, from: &str) -> TransportResult {
        for transport in &mut self.transports {
            if !transport.connected() || !transport.can_reach(target) {
                continue;
            }

            match transport.send(target, message, from) {
                Ok(true) => return TransportResult::success(transport.name()),
                Ok(false) => {}
                Err(err) => {
                    let classified = classify_error(Some(&err));
                    if !classified.retryable {
                        return TransportResult::failure(
                            transport.name(),
                            classified.reason,
                            classified.retryable,
                        );
                    }
                }
            }
        }
        TransportResult::failure("none", TransportFailureReason::Unreachable, false)
    }
}

/// Side-effect seam for the local tmux transport.
pub trait TmuxTransportIo {
    /// Send a message to a concrete tmux target.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific error string when tmux rejects delivery.
    fn send_to_tmux(&mut self, target: &str, message: &str) -> Result<(), String>;

    /// List local tmux sessions for oracle-name resolution.
    ///
    /// # Errors
    ///
    /// Returns an implementation-specific error string when session listing fails.
    fn list_tmux_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String>;

    /// Resolve an oracle query to a tmux target from already-listed sessions.
    fn find_tmux_window(
        &mut self,
        sessions: &[TmuxTransportSession],
        query: &str,
    ) -> Option<String>;
}

/// Portable local fast-path tmux transport.
pub struct TmuxLocalTransport<Io> {
    io: Io,
    connected: bool,
    message_handlers: usize,
    presence_handlers: usize,
    feed_handlers: usize,
}

impl<Io> TmuxLocalTransport<Io> {
    #[must_use]
    pub const fn new(io: Io) -> Self {
        Self {
            io,
            connected: false,
            message_handlers: 0,
            presence_handlers: 0,
            feed_handlers: 0,
        }
    }

    #[must_use]
    pub const fn connected(&self) -> bool {
        self.connected
    }

    pub const fn connect(&mut self) {
        self.connected = true;
    }

    pub const fn disconnect(&mut self) {
        self.connected = false;
    }

    pub const fn on_message(&mut self) {
        self.message_handlers += 1;
    }

    pub const fn on_presence(&mut self) {
        self.presence_handlers += 1;
    }

    pub const fn on_feed(&mut self) {
        self.feed_handlers += 1;
    }

    #[must_use]
    pub const fn handler_counts(&self) -> (usize, usize, usize) {
        (
            self.message_handlers,
            self.presence_handlers,
            self.feed_handlers,
        )
    }

    pub const fn publish_presence(&self) {}

    pub const fn publish_feed(&self) {}
}

impl<Io> TmuxLocalTransport<Io>
where
    Io: TmuxTransportIo,
{
    #[must_use]
    pub fn name(&self) -> &'static str {
        "tmux"
    }

    #[must_use]
    pub fn can_reach(&self, target: &TransportTarget) -> bool {
        is_local_host(target.host.as_deref())
    }

    /// Send using explicit `tmux_target` or by scanning sessions and resolving the oracle name.
    pub fn send(&mut self, target: &TransportTarget, message: &str) -> bool {
        if !self.can_reach(target) {
            return false;
        }
        let tmux_target = if let Some(tmux_target) = &target.tmux_target {
            tmux_target.clone()
        } else {
            let Ok(sessions) = self.io.list_tmux_sessions() else {
                return false;
            };
            let Some(resolved) = self.io.find_tmux_window(&sessions, &target.oracle) else {
                return false;
            };
            resolved
        };
        self.io.send_to_tmux(&tmux_target, message).is_ok()
    }

    #[must_use]
    pub const fn io(&self) -> &Io {
        &self.io
    }
}

/// Portable HTTP federation fallback transport.
pub struct HttpFederationTransport<Io> {
    config: HttpTransportConfig,
    io: Io,
    connected: bool,
    message_handlers: usize,
    presence_handlers: usize,
    feed_handlers: usize,
}

impl<Io> HttpFederationTransport<Io> {
    #[must_use]
    pub const fn new(config: HttpTransportConfig, io: Io) -> Self {
        Self {
            config,
            io,
            connected: false,
            message_handlers: 0,
            presence_handlers: 0,
            feed_handlers: 0,
        }
    }

    #[must_use]
    pub const fn connected(&self) -> bool {
        self.connected
    }

    pub fn connect(&mut self) {
        self.connected = !self.config.peers.is_empty();
    }

    pub const fn disconnect(&mut self) {
        self.connected = false;
    }

    pub const fn on_message(&mut self) {
        self.message_handlers += 1;
    }

    pub const fn on_presence(&mut self) {
        self.presence_handlers += 1;
    }

    pub const fn on_feed(&mut self) {
        self.feed_handlers += 1;
    }

    #[must_use]
    pub const fn handler_counts(&self) -> (usize, usize, usize) {
        (
            self.message_handlers,
            self.presence_handlers,
            self.feed_handlers,
        )
    }

    pub const fn publish_presence(&self) {}

    #[must_use]
    pub const fn io(&self) -> &Io {
        &self.io
    }
}

impl<Io> HttpFederationTransport<Io>
where
    Io: HttpTransportIo,
{
    #[must_use]
    pub fn name(&self) -> &'static str {
        "http-federation"
    }

    #[must_use]
    pub fn can_reach(&self, target: &TransportTarget) -> bool {
        !self.config.peers.is_empty() && !is_local_host(target.host.as_deref())
    }

    /// Send to the first remote sourced session whose window name contains the oracle query.
    pub fn send(&mut self, target: &TransportTarget, message: &str) -> bool {
        let Ok(local_sessions) = self.io.list_local_sessions() else {
            return false;
        };
        let Ok(all_sessions) = self.io.get_all_sessions(&local_sessions) else {
            return false;
        };
        let query = target.oracle.to_lowercase();
        for session in &all_sessions {
            let Some(source) = &session.source else {
                continue;
            };
            if source == "local" {
                continue;
            }
            let matches = session
                .windows
                .iter()
                .any(|window| window.name.to_lowercase().contains(&query));
            if !matches {
                continue;
            }
            let single = [session.clone()];
            let Some(tmux_target) = self.io.find_target_window(&single, &target.oracle) else {
                continue;
            };
            return self
                .io
                .send_peer_keys(source, &tmux_target, message)
                .unwrap_or(false);
        }
        false
    }

    /// Publish a feed event to every configured peer and return warnings for rejected posts.
    pub fn publish_feed(&mut self, event_json: &str) -> Vec<HttpFeedWarning> {
        let peers = self.config.peers.clone();
        let timeout = self.io.timeout_for("http");
        let mut warnings = Vec::new();
        for peer in peers {
            let url = format!("{peer}/api/feed");
            if let Err(reason) = self.io.post_peer_feed(&url, "POST", event_json, timeout) {
                warnings.push(HttpFeedWarning { peer, reason });
            }
        }
        warnings
    }
}

fn is_local_host(host: Option<&str>) -> bool {
    matches!(host, None | Some("local" | "localhost"))
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn rate_limit_like(msg: &str) -> bool {
    msg.contains("rate") && msg.contains("limit")
}

#[must_use]
pub fn classify_symmetric_federation_status(
    base: &FederationStatus,
    remote_statuses: &[(String, PeerFederationStatusResult)],
    local_node: &str,
) -> SymmetricFederationStatus {
    let pairs = base
        .peers
        .iter()
        .map(|peer| classify_peer_pair(base, remote_statuses, local_node, peer))
        .collect::<Vec<_>>();
    let healthy_pairs = pairs
        .iter()
        .filter(|pair| pair.pair == PairHealth::Healthy)
        .count();
    let total_pairs = pairs.len();

    SymmetricFederationStatus {
        local_url: base.local_url.clone(),
        local_node: local_node.to_owned(),
        pairs,
        healthy_pairs,
        total_pairs,
    }
}

fn classify_peer_pair(
    base: &FederationStatus,
    remote_statuses: &[(String, PeerFederationStatusResult)],
    local_node: &str,
    peer: &FederationPeerStatus,
) -> PairStatus {
    if !peer.reachable {
        return pair_status(
            peer,
            PairHealth::Down,
            false,
            None,
            Some("forward unreachable"),
        );
    }

    match remote_status_for(remote_statuses, &peer.url) {
        Some(PeerFederationStatusResult::Ok(status)) => {
            classify_ok_peer_view(base, local_node, peer, &status.peers)
        }
        Some(PeerFederationStatusResult::MissingPeers) => {
            classify_ok_peer_view(base, local_node, peer, &[])
        }
        Some(PeerFederationStatusResult::HttpStatus(status)) => pair_status(
            peer,
            PairHealth::Unknown,
            true,
            None,
            Some(format!("peer /api/federation/status returned {status}")),
        ),
        Some(PeerFederationStatusResult::FetchError(error)) => pair_status(
            peer,
            PairHealth::Unknown,
            true,
            None,
            Some(format!("peer status fetch failed: {error}")),
        ),
        None => pair_status(
            peer,
            PairHealth::Unknown,
            true,
            None,
            Some("peer /api/federation/status returned 0"),
        ),
    }
}

fn classify_ok_peer_view(
    base: &FederationStatus,
    local_node: &str,
    peer: &FederationPeerStatus,
    peer_peers: &[FederationPeerView],
) -> PairStatus {
    let local = peer_peers
        .iter()
        .find(|candidate| matches_local_peer(candidate, local_node, &base.local_url));

    let Some(local) = local else {
        return pair_status(
            peer,
            PairHealth::HalfUp,
            true,
            Some(false),
            Some("local node not in peer's peer list"),
        );
    };

    if local.reachable == Some(false) {
        return pair_status(
            peer,
            PairHealth::HalfUp,
            true,
            Some(false),
            Some("peer's view of local is unreachable"),
        );
    }

    pair_status(peer, PairHealth::Healthy, true, Some(true), None::<String>)
}

fn matches_local_peer(candidate: &FederationPeerView, local_node: &str, local_url: &str) -> bool {
    if candidate
        .node
        .as_deref()
        .is_some_and(|node| !local_node.is_empty() && node == local_node)
    {
        return true;
    }
    candidate.url.as_deref() == Some(local_url)
}

fn remote_status_for<'a>(
    remote_statuses: &'a [(String, PeerFederationStatusResult)],
    url: &str,
) -> Option<&'a PeerFederationStatusResult> {
    remote_statuses
        .iter()
        .find_map(|(peer_url, status)| (peer_url == url).then_some(status))
}

fn pair_status(
    peer: &FederationPeerStatus,
    pair: PairHealth,
    forward: bool,
    reverse: Option<bool>,
    reason: Option<impl Into<String>>,
) -> PairStatus {
    PairStatus {
        url: peer.url.clone(),
        node: peer.node.clone(),
        pair,
        forward,
        reverse,
        reason: reason.map(Into::into),
        latency: peer.latency,
        agents: peer.agents.clone(),
        clock_warning: peer.clock_warning,
    }
}

#[cfg(test)]
mod federation_symmetric_tests {
    use super::*;

    fn base(peers: Vec<FederationPeerStatus>) -> FederationStatus {
        FederationStatus {
            local_url: "http://localhost:3456".to_owned(),
            peers,
        }
    }

    fn peer(url: &str, reachable: bool, node: Option<&str>) -> FederationPeerStatus {
        FederationPeerStatus {
            url: url.to_owned(),
            node: node.map(str::to_owned),
            reachable,
            latency: Some(40),
            agents: Vec::new(),
            clock_warning: false,
        }
    }

    fn remote(peers: Vec<FederationPeerView>) -> PeerFederationStatusResult {
        PeerFederationStatusResult::Ok(PeerFederationStatus { peers })
    }

    fn view(url: &str, node: Option<&str>, reachable: bool) -> FederationPeerView {
        FederationPeerView {
            url: Some(url.to_owned()),
            node: node.map(str::to_owned),
            reachable: Some(reachable),
        }
    }

    #[test]
    fn no_peers_reports_empty_pair_counts() {
        let status = classify_symmetric_federation_status(&base(Vec::new()), &[], "white");

        assert!(status.pairs.is_empty());
        assert_eq!(status.total_pairs, 0);
        assert_eq!(status.healthy_pairs, 0);
        assert_eq!(status.local_node, "white");
    }

    #[test]
    fn reachable_peer_with_reachable_reverse_view_is_healthy() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[(
                "http://mba:3456".to_owned(),
                remote(vec![view("http://localhost:3456", Some("white"), true)]),
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Healthy);
        assert!(status.pairs[0].forward);
        assert_eq!(status.pairs[0].reverse, Some(true));
        assert_eq!(status.healthy_pairs, 1);
    }

    #[test]
    fn reachable_peer_missing_local_view_is_half_up() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[("http://mba:3456".to_owned(), remote(Vec::new()))],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::HalfUp);
        assert!(status.pairs[0].forward);
        assert_eq!(status.pairs[0].reverse, Some(false));
        assert!(status.pairs[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("not in peer")));
    }

    #[test]
    fn reachable_peer_marking_local_unreachable_is_half_up() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[(
                "http://mba:3456".to_owned(),
                remote(vec![view("http://localhost:3456", Some("white"), false)]),
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::HalfUp);
        assert!(status.pairs[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("unreachable")));
    }

    #[test]
    fn forward_unreachable_peer_is_down_without_reverse_check() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", false, None)]),
            &[],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Down);
        assert!(!status.pairs[0].forward);
        assert_eq!(status.pairs[0].reverse, None);
        assert_eq!(
            status.pairs[0].reason.as_deref(),
            Some("forward unreachable")
        );
    }

    #[test]
    fn non_ok_peer_status_is_unknown() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[(
                "http://mba:3456".to_owned(),
                PeerFederationStatusResult::HttpStatus(500),
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Unknown);
        assert!(status.pairs[0].forward);
        assert_eq!(status.pairs[0].reverse, None);
        assert!(status.pairs[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("returned 500")));
    }

    #[test]
    fn fetch_error_peer_status_is_unknown_with_reason() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[(
                "http://mba:3456".to_owned(),
                PeerFederationStatusResult::FetchError("network cable unplugged".to_owned()),
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Unknown);
        assert!(status.pairs[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("network cable")));
    }

    #[test]
    fn local_node_identity_match_wins_when_url_differs() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[(
                "http://mba:3456".to_owned(),
                remote(vec![view("http://10.0.0.1:3456", Some("white"), true)]),
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Healthy);
    }

    #[test]
    fn local_url_match_supports_legacy_peer_without_node_identity() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, None)]),
            &[(
                "http://mba:3456".to_owned(),
                remote(vec![view("http://localhost:3456", None, true)]),
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Healthy);
    }

    #[test]
    fn mixed_three_peer_mesh_counts_one_healthy_one_half_up_one_down() {
        let status = classify_symmetric_federation_status(
            &base(vec![
                peer("http://alpha:3456", true, Some("alpha")),
                peer("http://bravo:3456", true, Some("bravo")),
                peer("http://charlie:3456", false, None),
            ]),
            &[
                (
                    "http://alpha:3456".to_owned(),
                    remote(vec![view("http://localhost:3456", Some("white"), true)]),
                ),
                ("http://bravo:3456".to_owned(), remote(Vec::new())),
            ],
            "white",
        );
        let mut pair_healths = status
            .pairs
            .iter()
            .map(|pair| pair.pair)
            .collect::<Vec<_>>();
        pair_healths.sort_unstable_by_key(|state| state.as_str());

        assert_eq!(status.total_pairs, 3);
        assert_eq!(status.healthy_pairs, 1);
        assert_eq!(
            pair_healths,
            vec![PairHealth::Down, PairHealth::HalfUp, PairHealth::Healthy]
        );
    }

    #[test]
    fn peer_response_without_peers_field_is_half_up_defensively() {
        let status = classify_symmetric_federation_status(
            &base(vec![peer("http://mba:3456", true, Some("mba"))]),
            &[(
                "http://mba:3456".to_owned(),
                PeerFederationStatusResult::MissingPeers,
            )],
            "white",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::HalfUp);
    }
}

#[cfg(test)]
mod tmux_transport_tests {
    use super::*;

    #[derive(Default)]
    struct FakeTmuxIo {
        sends: Vec<(String, String)>,
        scanned: bool,
        sessions: Vec<TmuxTransportSession>,
        queries: Vec<String>,
        find_result: Option<String>,
        send_error: bool,
    }

    impl TmuxTransportIo for FakeTmuxIo {
        fn send_to_tmux(&mut self, target: &str, message: &str) -> Result<(), String> {
            if self.send_error {
                return Err("tmux rejected".to_owned());
            }
            self.sends.push((target.to_owned(), message.to_owned()));
            Ok(())
        }

        fn list_tmux_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String> {
            self.scanned = true;
            Ok(self.sessions.clone())
        }

        fn find_tmux_window(
            &mut self,
            sessions: &[TmuxTransportSession],
            query: &str,
        ) -> Option<String> {
            assert_eq!(sessions, self.sessions.as_slice());
            self.queries.push(query.to_owned());
            self.find_result.clone()
        }
    }

    fn sample_sessions() -> Vec<TmuxTransportSession> {
        vec![TmuxTransportSession {
            name: "47-mawjs".to_owned(),
            windows: vec![
                TmuxTransportWindow {
                    index: 0,
                    name: "mawjs-oracle".to_owned(),
                    active: true,
                },
                TmuxTransportWindow {
                    index: 1,
                    name: "mawjs-codex".to_owned(),
                    active: false,
                },
            ],
        }]
    }

    #[test]
    fn tmux_transport_tracks_local_lifecycle_and_reachability() {
        let mut transport = TmuxLocalTransport::new(FakeTmuxIo::default());
        assert_eq!(transport.name(), "tmux");
        assert!(!transport.connected());
        transport.connect();
        assert!(transport.connected());
        transport.disconnect();
        assert!(!transport.connected());

        assert!(transport.can_reach(&TransportTarget {
            oracle: "mawjs".to_owned(),
            host: None,
            tmux_target: None,
        }));
        assert!(transport.can_reach(&TransportTarget {
            oracle: "mawjs".to_owned(),
            host: Some("local".to_owned()),
            tmux_target: None,
        }));
        assert!(transport.can_reach(&TransportTarget {
            oracle: "mawjs".to_owned(),
            host: Some("localhost".to_owned()),
            tmux_target: None,
        }));
        assert!(!transport.can_reach(&TransportTarget {
            oracle: "mawjs".to_owned(),
            host: Some("m5".to_owned()),
            tmux_target: None,
        }));
    }

    #[test]
    fn tmux_transport_uses_explicit_target_without_scanning() {
        let mut transport = TmuxLocalTransport::new(FakeTmuxIo::default());
        assert!(transport.send(
            &TransportTarget {
                oracle: "ignored".to_owned(),
                host: None,
                tmux_target: Some("47-mawjs:1".to_owned()),
            },
            "hello",
        ));
        assert!(!transport.io().scanned);
        assert_eq!(
            transport.io().sends,
            vec![("47-mawjs:1".to_owned(), "hello".to_owned())]
        );
    }

    #[test]
    fn tmux_transport_resolves_local_oracle_through_session_scan() {
        let io = FakeTmuxIo {
            sessions: sample_sessions(),
            find_result: Some("47-mawjs:1".to_owned()),
            ..FakeTmuxIo::default()
        };
        let mut transport = TmuxLocalTransport::new(io);
        assert!(transport.send(
            &TransportTarget {
                oracle: "mawjs-codex".to_owned(),
                host: None,
                tmux_target: None,
            },
            "ping",
        ));
        assert!(transport.io().scanned);
        assert_eq!(transport.io().queries, vec!["mawjs-codex".to_owned()]);
        assert_eq!(
            transport.io().sends,
            vec![("47-mawjs:1".to_owned(), "ping".to_owned())]
        );
    }

    #[test]
    fn tmux_transport_returns_false_for_remote_unresolved_and_throwing_paths() {
        let mut remote = TmuxLocalTransport::new(FakeTmuxIo {
            sessions: sample_sessions(),
            ..FakeTmuxIo::default()
        });
        assert!(!remote.send(
            &TransportTarget {
                oracle: "mawjs".to_owned(),
                host: Some("remote".to_owned()),
                tmux_target: None,
            },
            "nope",
        ));
        assert!(remote.io().sends.is_empty());

        let mut unresolved = TmuxLocalTransport::new(FakeTmuxIo {
            sessions: sample_sessions(),
            find_result: None,
            ..FakeTmuxIo::default()
        });
        assert!(!unresolved.send(
            &TransportTarget {
                oracle: "missing".to_owned(),
                host: None,
                tmux_target: None,
            },
            "nope",
        ));

        let mut throwing = TmuxLocalTransport::new(FakeTmuxIo {
            sessions: sample_sessions(),
            find_result: Some("47-mawjs:1".to_owned()),
            send_error: true,
            ..FakeTmuxIo::default()
        });
        assert!(!throwing.send(
            &TransportTarget {
                oracle: "mawjs".to_owned(),
                host: None,
                tmux_target: None,
            },
            "nope",
        ));
        assert!(throwing.io().sends.is_empty());
    }

    #[test]
    fn tmux_transport_accepts_handlers_and_ignores_publish_hooks() {
        let mut transport = TmuxLocalTransport::new(FakeTmuxIo::default());
        transport.on_message();
        transport.on_presence();
        transport.on_feed();
        assert_eq!(transport.handler_counts(), (1, 1, 1));
        transport.publish_presence();
        transport.publish_feed();
    }
}

#[cfg(test)]
mod http_transport_tests {
    use super::*;

    #[derive(Default)]
    struct FakeHttpIo {
        local_sessions: Vec<TmuxTransportSession>,
        all_sessions: Vec<TransportSession>,
        sent: Vec<(String, String, String)>,
        posts: Vec<(String, String, String, u64)>,
        queries: Vec<String>,
        find_result: Option<String>,
        fail_post_url: Option<String>,
    }

    impl HttpTransportIo for FakeHttpIo {
        fn list_local_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String> {
            Ok(self.local_sessions.clone())
        }

        fn get_all_sessions(
            &mut self,
            local_sessions: &[TmuxTransportSession],
        ) -> Result<Vec<TransportSession>, String> {
            assert_eq!(local_sessions, self.local_sessions.as_slice());
            Ok(self.all_sessions.clone())
        }

        fn find_target_window(
            &mut self,
            sessions: &[TransportSession],
            query: &str,
        ) -> Option<String> {
            assert_eq!(sessions.len(), 1);
            self.queries.push(query.to_owned());
            self.find_result.clone()
        }

        fn send_peer_keys(
            &mut self,
            source: &str,
            target: &str,
            message: &str,
        ) -> Result<bool, String> {
            self.sent
                .push((source.to_owned(), target.to_owned(), message.to_owned()));
            Ok(true)
        }

        fn post_peer_feed(
            &mut self,
            url: &str,
            method: &str,
            body: &str,
            timeout_ms: u64,
        ) -> Result<HttpPostResult, String> {
            self.posts.push((
                url.to_owned(),
                method.to_owned(),
                body.to_owned(),
                timeout_ms,
            ));
            if self.fail_post_url.as_deref() == Some(url) {
                Err("boom".to_owned())
            } else {
                Ok(HttpPostResult {
                    ok: true,
                    status: 200,
                })
            }
        }

        fn timeout_for(&self, transport: &str) -> u64 {
            assert_eq!(transport, "http");
            1234
        }
    }

    fn window(name: &str) -> TmuxTransportWindow {
        TmuxTransportWindow {
            index: 0,
            name: name.to_owned(),
            active: true,
        }
    }

    fn local_session(name: &str, window_name: &str) -> TmuxTransportSession {
        TmuxTransportSession {
            name: name.to_owned(),
            windows: vec![window(window_name)],
        }
    }

    fn sourced_session(name: &str, window_name: &str, source: Option<&str>) -> TransportSession {
        TransportSession {
            name: name.to_owned(),
            source: source.map(str::to_owned),
            windows: vec![window(window_name)],
        }
    }

    #[test]
    fn http_transport_connects_only_when_peers_are_configured() {
        let mut offline = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: Vec::new(),
                self_host: "local".to_owned(),
            },
            FakeHttpIo::default(),
        );
        assert_eq!(offline.name(), "http-federation");
        assert!(!offline.connected());
        offline.connect();
        assert!(!offline.connected());

        let mut online = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: vec!["http://peer".to_owned()],
                self_host: "local".to_owned(),
            },
            FakeHttpIo::default(),
        );
        online.connect();
        assert!(online.connected());
        online.disconnect();
        assert!(!online.connected());
    }

    #[test]
    fn http_transport_can_reach_only_remote_targets_when_peers_exist() {
        let no_peers = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: Vec::new(),
                self_host: "local".to_owned(),
            },
            FakeHttpIo::default(),
        );
        assert!(!no_peers.can_reach(&TransportTarget {
            oracle: "mawjs".to_owned(),
            host: Some("m5".to_owned()),
            tmux_target: None,
        }));

        let transport = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: vec!["http://peer".to_owned()],
                self_host: "local".to_owned(),
            },
            FakeHttpIo::default(),
        );
        for host in [None, Some("local"), Some("localhost")] {
            assert!(!transport.can_reach(&TransportTarget {
                oracle: "mawjs".to_owned(),
                host: host.map(str::to_owned),
                tmux_target: None,
            }));
        }
        assert!(transport.can_reach(&TransportTarget {
            oracle: "mawjs".to_owned(),
            host: Some("m5".to_owned()),
            tmux_target: None,
        }));
    }

    #[test]
    fn http_transport_sends_through_peer_that_owns_matching_window() {
        let local_sessions = vec![local_session("local", "local-oracle")];
        let all_sessions = vec![
            sourced_session("local", "local-oracle", Some("local")),
            sourced_session("remote-a", "other-oracle", Some("http://peer-a")),
            sourced_session("remote-b", "target-oracle", Some("http://peer-b")),
        ];
        let io = FakeHttpIo {
            local_sessions,
            all_sessions,
            find_result: Some("remote-b:0".to_owned()),
            ..FakeHttpIo::default()
        };
        let mut transport = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: vec!["http://peer-a".to_owned(), "http://peer-b".to_owned()],
                self_host: "local".to_owned(),
            },
            io,
        );
        assert!(transport.send(
            &TransportTarget {
                oracle: "target".to_owned(),
                host: Some("remote".to_owned()),
                tmux_target: None,
            },
            "hello",
        ));
        assert_eq!(transport.io().queries, vec!["target".to_owned()]);
        assert_eq!(
            transport.io().sent,
            vec![(
                "http://peer-b".to_owned(),
                "remote-b:0".to_owned(),
                "hello".to_owned(),
            ),]
        );
    }

    #[test]
    fn http_transport_returns_false_when_no_remote_session_resolves() {
        let io = FakeHttpIo {
            all_sessions: vec![
                sourced_session("local", "target-oracle", None),
                sourced_session("remote-a", "other-oracle", Some("http://peer-a")),
                sourced_session("remote-b", "target-oracle", Some("http://peer-b")),
            ],
            find_result: None,
            ..FakeHttpIo::default()
        };
        let mut transport = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: vec!["http://peer".to_owned()],
                self_host: "local".to_owned(),
            },
            io,
        );
        assert!(!transport.send(
            &TransportTarget {
                oracle: "target".to_owned(),
                host: Some("remote".to_owned()),
                tmux_target: None,
            },
            "hello",
        ));
        assert!(transport.io().sent.is_empty());
    }

    #[test]
    fn http_transport_publishes_feed_events_to_every_peer_and_warns_on_rejections() {
        let mut transport = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: vec![
                    "http://a".to_owned(),
                    "http://b".to_owned(),
                    "http://c".to_owned(),
                ],
                self_host: "local".to_owned(),
            },
            FakeHttpIo {
                fail_post_url: Some("http://b/api/feed".to_owned()),
                ..FakeHttpIo::default()
            },
        );
        let warnings = transport.publish_feed("{\"message\":\"hello\"}");
        assert_eq!(
            transport.io().posts,
            vec![
                (
                    "http://a/api/feed".to_owned(),
                    "POST".to_owned(),
                    "{\"message\":\"hello\"}".to_owned(),
                    1234,
                ),
                (
                    "http://b/api/feed".to_owned(),
                    "POST".to_owned(),
                    "{\"message\":\"hello\"}".to_owned(),
                    1234,
                ),
                (
                    "http://c/api/feed".to_owned(),
                    "POST".to_owned(),
                    "{\"message\":\"hello\"}".to_owned(),
                    1234,
                ),
            ]
        );
        assert_eq!(
            warnings,
            vec![HttpFeedWarning {
                peer: "http://b".to_owned(),
                reason: "boom".to_owned(),
            }]
        );
    }

    #[test]
    fn http_transport_accepts_handlers_and_ignores_presence() {
        let mut transport = HttpFederationTransport::new(
            HttpTransportConfig {
                peers: Vec::new(),
                self_host: "local".to_owned(),
            },
            FakeHttpIo::default(),
        );
        transport.on_message();
        transport.on_presence();
        transport.on_feed();
        assert_eq!(transport.handler_counts(), (1, 1, 1));
        transport.publish_presence();
    }
}

#[cfg(test)]
mod coverage_gap_tests {
    use super::*;

    struct FailingTmuxListIo;

    impl TmuxTransportIo for FailingTmuxListIo {
        fn send_to_tmux(&mut self, _target: &str, _message: &str) -> Result<(), String> {
            Ok(())
        }

        fn list_tmux_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String> {
            Err("tmux list failed".to_owned())
        }

        fn find_tmux_window(
            &mut self,
            _sessions: &[TmuxTransportSession],
            _query: &str,
        ) -> Option<String> {
            Some("ignored:0".to_owned())
        }
    }

    #[derive(Default)]
    struct FailingHttpIo {
        fail_all_sessions: bool,
    }

    impl HttpTransportIo for FailingHttpIo {
        fn list_local_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String> {
            if self.fail_all_sessions {
                Ok(Vec::new())
            } else {
                Err("local session list failed".to_owned())
            }
        }

        fn get_all_sessions(
            &mut self,
            _local_sessions: &[TmuxTransportSession],
        ) -> Result<Vec<TransportSession>, String> {
            Err("aggregate failed".to_owned())
        }

        fn find_target_window(
            &mut self,
            _sessions: &[TransportSession],
            _query: &str,
        ) -> Option<String> {
            Some("ignored:0".to_owned())
        }

        fn send_peer_keys(
            &mut self,
            _source: &str,
            _target: &str,
            _message: &str,
        ) -> Result<bool, String> {
            Ok(true)
        }

        fn post_peer_feed(
            &mut self,
            _url: &str,
            _method: &str,
            _body: &str,
            _timeout_ms: u64,
        ) -> Result<HttpPostResult, String> {
            Ok(HttpPostResult {
                ok: true,
                status: 200,
            })
        }

        fn timeout_for(&self, _transport: &str) -> u64 {
            1
        }
    }

    fn target(oracle: &str) -> TransportTarget {
        TransportTarget {
            oracle: oracle.to_owned(),
            host: Some("remote".to_owned()),
            tmux_target: None,
        }
    }

    #[test]
    fn fake_ios_exercise_all_required_trait_methods() {
        let mut tmux = FailingTmuxListIo;
        assert!(tmux.send_to_tmux("target", "message").is_ok());
        assert_eq!(
            tmux.find_tmux_window(&[], "mawjs"),
            Some("ignored:0".to_owned())
        );

        let mut http = FailingHttpIo::default();
        assert_eq!(
            http.find_target_window(&[], "mawjs"),
            Some("ignored:0".to_owned())
        );
        assert_eq!(http.send_peer_keys("source", "target", "message"), Ok(true));
        assert_eq!(
            http.post_peer_feed("http://peer/api/feed", "POST", "{}", 1),
            Ok(HttpPostResult {
                ok: true,
                status: 200,
            })
        );
        assert_eq!(http.timeout_for("http"), 1);
    }

    #[test]
    fn failure_reason_and_pair_health_labels_are_stable() {
        assert_eq!(TransportFailureReason::Timeout.as_str(), "timeout");
        assert_eq!(TransportFailureReason::Unreachable.as_str(), "unreachable");
        assert_eq!(TransportFailureReason::Auth.as_str(), "auth");
        assert_eq!(TransportFailureReason::RateLimit.as_str(), "rate_limit");
        assert_eq!(TransportFailureReason::Rejected.as_str(), "rejected");
        assert_eq!(TransportFailureReason::ParseError.as_str(), "parse_error");
        assert_eq!(TransportFailureReason::Unknown.as_str(), "unknown");
        assert_eq!(PairHealth::Unknown.as_str(), "unknown");
    }

    #[test]
    fn unknown_error_strings_remain_non_retryable_unknowns() {
        assert_eq!(
            classify_error(Some("socket evaporated mysteriously")),
            ClassifiedError {
                reason: TransportFailureReason::Unknown,
                retryable: false,
            }
        );
    }

    #[test]
    fn tmux_session_conversion_preserves_windows_with_no_source() {
        let local = TmuxTransportSession {
            name: "mawjs".to_owned(),
            windows: vec![TmuxTransportWindow {
                index: 2,
                name: "oracle".to_owned(),
                active: false,
            }],
        };

        let session = TransportSession::from(local.clone());

        assert_eq!(session.name, local.name);
        assert_eq!(session.source, None);
        assert_eq!(session.windows, local.windows);
    }

    #[test]
    fn tmux_transport_returns_false_when_session_listing_fails() {
        let mut transport = TmuxLocalTransport::new(FailingTmuxListIo);

        assert!(!transport.send(
            &TransportTarget {
                oracle: "mawjs".to_owned(),
                host: None,
                tmux_target: None,
            },
            "hello",
        ));
    }

    #[test]
    fn http_transport_returns_false_when_session_collection_fails() {
        let config = HttpTransportConfig {
            peers: vec!["http://peer".to_owned()],
            self_host: "local".to_owned(),
        };
        let mut list_fails = HttpFederationTransport::new(config.clone(), FailingHttpIo::default());
        assert!(!list_fails.send(&target("mawjs"), "hello"));

        let mut aggregate_fails = HttpFederationTransport::new(
            config,
            FailingHttpIo {
                fail_all_sessions: true,
            },
        );
        assert!(!aggregate_fails.send(&target("mawjs"), "hello"));
    }

    #[test]
    fn missing_remote_status_is_unknown_with_zero_status_reason() {
        let status = classify_symmetric_federation_status(
            &FederationStatus {
                local_url: "http://local:3456".to_owned(),
                peers: vec![FederationPeerStatus {
                    url: "http://peer:3456".to_owned(),
                    node: Some("peer".to_owned()),
                    reachable: true,
                    latency: None,
                    agents: vec!["mawjs".to_owned()],
                    clock_warning: true,
                }],
            },
            &[],
            "local",
        );

        assert_eq!(status.pairs[0].pair, PairHealth::Unknown);
        assert_eq!(
            status.pairs[0].reason.as_deref(),
            Some("peer /api/federation/status returned 0")
        );
        assert_eq!(status.pairs[0].agents, ["mawjs"]);
        assert!(status.pairs[0].clock_warning);
    }
}
