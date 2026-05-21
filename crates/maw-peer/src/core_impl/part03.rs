/// Evaluate and persist a peer identity TOFU decision.
///
/// # Errors
///
/// Returns mismatch or peer-store IO failures from [`apply_tofu_decision`].
pub fn tofu_record_peer_identity(
    env: &PeerStoreEnv,
    alias: &str,
    peer: Option<&PeerRecord>,
    observed: Option<&str>,
    now: &str,
) -> Result<TofuDecision, TofuApplyError> {
    let decision = evaluate_peer_identity(alias, peer, observed);
    apply_tofu_decision(env, &decision, now)?;
    Ok(decision)
}

/// Clear a cached pubkey for `alias`.
///
/// # Errors
///
/// Returns peer-store mutation write failures.
pub fn forget_peer_pubkey(env: &PeerStoreEnv, alias: &str) -> io::Result<&'static str> {
    let mut outcome = "not-found";
    mutate_peer_store(env, |data| {
        let Some(peer) = data.peers.get_mut(alias) else {
            outcome = "not-found";
            return;
        };
        if peer.pubkey.is_none() {
            outcome = "no-pubkey";
            return;
        }
        peer.pubkey = None;
        peer.pubkey_first_seen = None;
        outcome = "cleared";
    })?;
    Ok(outcome)
}

#[derive(Debug)]
pub enum TofuApplyError {
    Io(io::Error),
    Mismatch(PeerPubkeyMismatchError),
}

impl std::fmt::Display for TofuApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(f),
            Self::Mismatch(error) => error.fmt(f),
        }
    }
}

impl Error for TofuApplyError {}

impl From<io::Error> for TofuApplyError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<PeerPubkeyMismatchError> for TofuApplyError {
    fn from(value: PeerPubkeyMismatchError) -> Self {
        Self::Mismatch(value)
    }
}

/// Deterministic input for maw-js `cmdAdd` peer-cache behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAddPlan {
    pub alias: String,
    pub url: String,
    pub node: Option<String>,
    pub authenticated_pubkey: Option<String>,
    pub authenticated_identity: Option<PeerIdentity>,
    pub mark_symmetric_check: bool,
    pub one_way: Option<bool>,
    pub now: String,
    pub peers: BTreeMap<String, PeerRecord>,
    pub probe: ProbePeerResult,
}

/// Deterministic result for maw-js `cmdAdd`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAddResult {
    pub alias: String,
    pub overwrote: bool,
    pub peer: PeerRecord,
    pub probe_error: Option<ProbeLastError>,
    pub pubkey_mismatch: Option<PeerPubkeyMismatchError>,
    pub peers_after: BTreeMap<String, PeerRecord>,
}

/// Port of maw-js `cmdAdd` cache/TOFU behavior over deterministic inputs.
///
/// # Errors
///
/// Returns maw-js-compatible alias or URL validation failures.
pub fn cmd_peer_add_from_plan(plan: &PeerAddPlan) -> Result<PeerAddResult, String> {
    if let Some(message) = validate_peer_alias(&plan.alias) {
        return Err(message);
    }
    if let Some(message) = validate_peer_url(&plan.url) {
        return Err(message);
    }

    let observed_pubkey = plan
        .authenticated_pubkey
        .as_deref()
        .or(plan.probe.pubkey.as_deref());
    let existing = plan.peers.get(&plan.alias);
    if let (Some(authenticated), Some(probed)) = (
        plan.authenticated_pubkey.as_deref(),
        plan.probe.pubkey.as_deref(),
    ) {
        if authenticated != probed {
            return Ok(peer_add_mismatch_result(
                plan,
                existing,
                authenticated,
                probed,
            ));
        }
    }
    let tofu_decision = evaluate_peer_identity(&plan.alias, existing, observed_pubkey);
    if tofu_decision.kind == TofuDecisionKind::Mismatch {
        let cached = tofu_decision.cached.unwrap_or_default();
        let observed = tofu_decision.observed.unwrap_or_default();
        return Ok(peer_add_mismatch_result(plan, existing, &cached, &observed));
    }

    let mut peer = peer_add_new_record(plan);
    if let Some(existing) = existing {
        peer_add_apply_existing(plan, existing, &tofu_decision, &mut peer);
    } else if tofu_decision.kind == TofuDecisionKind::TofuBootstrap {
        peer.pubkey.clone_from(&tofu_decision.observed);
        peer.pubkey_first_seen = Some(plan.now.clone());
    }
    if existing.is_none() && plan.mark_symmetric_check {
        peer.last_symmetric_check = Some(plan.now.clone());
        peer.one_way = Some(plan.one_way.unwrap_or(plan.probe.error.is_some()));
    }

    let overwrote = plan.peers.contains_key(&plan.alias);
    let mut peers_after = plan.peers.clone();
    peers_after.insert(plan.alias.clone(), peer.clone());

    Ok(PeerAddResult {
        alias: plan.alias.clone(),
        overwrote,
        peer,
        probe_error: plan.probe.error.clone(),
        pubkey_mismatch: None,
        peers_after,
    })
}

fn peer_add_new_record(plan: &PeerAddPlan) -> PeerRecord {
    PeerRecord {
        url: plan.url.clone(),
        node: plan.node.clone().or_else(|| plan.probe.node.clone()),
        added_at: plan.now.clone(),
        last_seen: plan.probe.error.is_none().then(|| plan.now.clone()),
        last_error: plan.probe.error.clone(),
        nickname: plan.probe.nickname.clone(),
        pubkey: None,
        pubkey_first_seen: None,
        identity: plan
            .probe
            .identity
            .clone()
            .or_else(|| plan.authenticated_identity.clone()),
        one_way: None,
        last_symmetric_check: None,
    }
}

fn peer_add_apply_existing(
    plan: &PeerAddPlan,
    existing: &PeerRecord,
    tofu_decision: &TofuDecision,
    peer: &mut PeerRecord,
) {
    if existing
        .pubkey
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        peer.pubkey.clone_from(&existing.pubkey);
        peer.pubkey_first_seen
            .clone_from(&existing.pubkey_first_seen);
    } else if tofu_decision.kind == TofuDecisionKind::TofuBootstrap {
        peer.pubkey.clone_from(&tofu_decision.observed);
        peer.pubkey_first_seen = Some(plan.now.clone());
    }
    if peer.identity.is_none() {
        peer.identity.clone_from(&existing.identity);
    }
    if plan.mark_symmetric_check {
        peer.last_symmetric_check = Some(plan.now.clone());
        peer.one_way = Some(plan.one_way.unwrap_or(plan.probe.error.is_some()));
    } else if existing.last_symmetric_check.is_some() {
        peer.last_symmetric_check
            .clone_from(&existing.last_symmetric_check);
        peer.one_way = existing.one_way;
    }
}

fn peer_add_mismatch_result(
    plan: &PeerAddPlan,
    existing: Option<&PeerRecord>,
    cached: &str,
    observed: &str,
) -> PeerAddResult {
    PeerAddResult {
        alias: plan.alias.clone(),
        overwrote: existing.is_some(),
        peer: existing
            .cloned()
            .unwrap_or_else(|| peer_add_new_record(plan)),
        probe_error: plan.probe.error.clone(),
        pubkey_mismatch: Some(PeerPubkeyMismatchError::new(
            plan.alias.clone(),
            cached,
            observed,
        )),
        peers_after: plan.peers.clone(),
    }
}

/// Deterministic input for maw-js `cmdProbe` peer-cache behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerProbePlan {
    pub alias: String,
    pub now: String,
    pub peers: BTreeMap<String, PeerRecord>,
    pub probe: ProbePeerResult,
    pub remove_before_mutate: bool,
}

/// Deterministic result for maw-js `cmdProbe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerProbeResult {
    pub alias: String,
    pub url: String,
    pub node: Option<String>,
    pub ok: bool,
    pub error: Option<ProbeLastError>,
    pub pubkey_mismatch: Option<PeerPubkeyMismatchError>,
    pub peers_after: BTreeMap<String, PeerRecord>,
}

/// Port of maw-js `cmdProbe` cache/TOFU behavior over deterministic inputs.
///
/// # Errors
///
/// Returns when the alias is not present in the input peer store.
pub fn cmd_peer_probe_from_plan(plan: &PeerProbePlan) -> Result<PeerProbeResult, String> {
    let Some(existing) = plan.peers.get(&plan.alias) else {
        return Err(format!("peer \"{}\" not found", plan.alias));
    };

    let tofu_decision =
        evaluate_peer_identity(&plan.alias, Some(existing), plan.probe.pubkey.as_deref());
    if tofu_decision.kind == TofuDecisionKind::Mismatch {
        return Ok(PeerProbeResult {
            alias: plan.alias.clone(),
            url: existing.url.clone(),
            node: plan.probe.node.clone().or_else(|| existing.node.clone()),
            ok: false,
            error: plan.probe.error.clone(),
            pubkey_mismatch: Some(PeerPubkeyMismatchError::new(
                plan.alias.clone(),
                tofu_decision.cached.unwrap_or_default(),
                tofu_decision.observed.unwrap_or_default(),
            )),
            peers_after: plan.peers.clone(),
        });
    }

    let mut peers_after = plan.peers.clone();
    if plan.remove_before_mutate {
        peers_after.remove(&plan.alias);
    }
    if let Some(peer) = peers_after.get_mut(&plan.alias) {
        if let Some(error) = &plan.probe.error {
            peer.last_error = Some(error.clone());
        } else {
            peer.last_error = None;
            peer.last_seen = Some(plan.now.clone());
            if let Some(node) = &plan.probe.node {
                peer.node = Some(node.clone());
            }
            if let Some(nickname) = &plan.probe.nickname {
                peer.nickname = Some(nickname.clone());
            }
            if let Some(identity) = &plan.probe.identity {
                peer.identity = Some(identity.clone());
            }
        }
        if tofu_decision.kind == TofuDecisionKind::TofuBootstrap
            && peer.pubkey.as_deref().is_none_or(str::is_empty)
        {
            peer.pubkey.clone_from(&tofu_decision.observed);
            peer.pubkey_first_seen = Some(plan.now.clone());
        }
    }

    Ok(PeerProbeResult {
        alias: plan.alias.clone(),
        url: existing.url.clone(),
        node: plan.probe.node.clone().or_else(|| existing.node.clone()),
        ok: plan.probe.error.is_none(),
        error: plan.probe.error.clone(),
        pubkey_mismatch: None,
        peers_after,
    })
}

fn read_peer_store_unlocked(path: &Path) -> PeerStoreFile {
    if !path.exists() {
        return empty_peer_store();
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return empty_peer_store();
    };
    parse_peer_store(&raw).unwrap_or_else(|_| empty_peer_store())
}

fn parse_peer_store(raw: &str) -> Result<PeerStoreFile, String> {
    let value = serde_json::from_str::<serde_json::Value>(raw).map_err(|err| err.to_string())?;
    let peers = match value.get("peers") {
        Some(peers) if peers.is_object() => peers.clone(),
        Some(_) => {
            return Err("invalid store shape (expected { peers: { ... } } object)".to_owned());
        }
        None => serde_json::json!({}),
    };
    serde_json::from_value(serde_json::json!({ "version": 1, "peers": peers }))
        .map_err(|err| err.to_string())
}

fn write_peer_store_atomic(path: &Path, data: &PeerStoreFile) -> io::Result<()> {
    let tmp = tmp_peer_store_path(path);
    let json = serde_json::to_string_pretty(data).map_err(io::Error::other)?;
    fs::write(&tmp, format!("{json}\n"))?;
    fs::rename(tmp, path)
}

fn tmp_peer_store_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.tmp", path.display()))
}

fn corrupt_peer_store_path(path: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    PathBuf::from(format!("{}.corrupt-{stamp}", path.display()))
}

fn prefix16(value: &str) -> &str {
    value.get(..16).unwrap_or(value)
}

/// Deterministic input for maw-js `cmdProbeAll`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAllPlan {
    pub timeout_ms: u64,
    pub now: String,
    pub peers: Vec<(String, PeerRecord)>,
    /// URL → probe result → elapsed milliseconds.
    pub probe_results: Vec<(String, ProbePeerResult, u64)>,
    /// Aliases removed after load and before mutation.
    pub removed_before_mutate: Vec<String>,
}

/// Renderable per-peer probe-all row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAllRow {
    pub alias: String,
    pub url: String,
    pub node: Option<String>,
    pub last_seen: Option<String>,
    pub ok: bool,
    pub ms: u64,
    pub error: Option<ProbeLastError>,
}

/// Deterministic result for maw-js `cmdProbeAll`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAllResult {
    pub rows: Vec<ProbeAllRow>,
    pub ok_count: usize,
    pub fail_count: usize,
    pub worst_exit_code: i32,
    pub probe_calls: Vec<(String, u64)>,
    pub mutate_calls: usize,
    pub peers_after: BTreeMap<String, PeerRecord>,
}

/// Port of maw-js `cmdProbeAll` over deterministic store/probe inputs.
#[must_use]
pub fn probe_all_from_plan(plan: &ProbeAllPlan) -> ProbeAllResult {
    let mut peers_after: BTreeMap<String, PeerRecord> = plan.peers.iter().cloned().collect();
    let probe_results: BTreeMap<String, (ProbePeerResult, u64)> = plan
        .probe_results
        .iter()
        .map(|(url, result, ms)| (url.clone(), (result.clone(), *ms)))
        .collect();
    let mut entries = plan.peers.clone();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut probe_calls = Vec::with_capacity(entries.len());
    let mut rows = Vec::with_capacity(entries.len());
    for (alias, peer) in entries {
        probe_calls.push((peer.url.clone(), plan.timeout_ms));
        let (probe, ms) = probe_results
            .get(&peer.url)
            .cloned()
            .unwrap_or_else(|| (probe_failure_without_error(), 0));
        let error = probe.error.clone();
        rows.push(ProbeAllRow {
            alias,
            url: peer.url,
            node: probe.node.or(peer.node),
            last_seen: peer.last_seen,
            ok: error.is_none(),
            ms,
            error,
        });
    }

    let mutate_calls = usize::from(!rows.is_empty());
    if mutate_calls == 1 {
        for alias in &plan.removed_before_mutate {
            peers_after.remove(alias);
        }
        for row in &mut rows {
            let Some(peer) = peers_after.get_mut(&row.alias) else {
                continue;
            };
            if row.ok {
                peer.last_error = None;
                peer.last_seen = Some(plan.now.clone());
                row.last_seen = Some(plan.now.clone());
                if let Some(node) = &row.node {
                    peer.node = Some(node.clone());
                }
            } else if let Some(error) = &row.error {
                peer.last_error = Some(error.clone());
            }
        }
    }

    let ok_count = rows.iter().filter(|row| row.ok).count();
    let fail_count = rows.len() - ok_count;
    let worst_exit_code = rows
        .iter()
        .filter_map(|row| row.error.as_ref())
        .map(|err| probe_exit_code(err.code))
        .max()
        .unwrap_or(0);

    ProbeAllResult {
        rows,
        ok_count,
        fail_count,
        worst_exit_code,
        probe_calls,
        mutate_calls,
        peers_after,
    }
}

fn probe_failure_without_error() -> ProbePeerResult {
    ProbePeerResult {
        node: None,
        nickname: None,
        pubkey: None,
        identity: None,
        error: None,
    }
}

