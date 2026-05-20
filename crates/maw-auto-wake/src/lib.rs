//! Pure auto-wake policy ported from maw-js `should-auto-wake.ts`.
//!
//! This crate intentionally has no I/O. Callers provide site-local facts and
//! receive the same `{ wake, reason }` decision shape used by maw-js.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoWakeSite {
    View,
    Hey,
    ApiSend,
    ApiWake,
    Peek,
    Bud,
    WakeCmd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoWakeManifest {
    pub name: String,
    pub sources: Vec<String>,
    pub is_live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoWakeOptions {
    pub site: AutoWakeSite,
    pub is_live: Option<bool>,
    pub is_fleet_known: Option<bool>,
    pub force: bool,
    pub no_wake: bool,
    pub is_canonical_target: bool,
    pub manifest: Option<AutoWakeManifest>,
}

impl Default for AutoWakeOptions {
    fn default() -> Self {
        Self {
            site: AutoWakeSite::View,
            is_live: None,
            is_fleet_known: None,
            force: false,
            no_wake: false,
            is_canonical_target: false,
            manifest: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoWakeDecision {
    pub wake: bool,
    pub reason: String,
}

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn should_auto_wake(_oracle: &str, opts: AutoWakeOptions) -> AutoWakeDecision {
    let mut is_fleet_known = opts.is_fleet_known;
    let mut is_live = opts.is_live;
    if let Some(manifest) = opts.manifest.as_ref() {
        is_fleet_known = Some(manifest.sources.iter().any(|source| source == "fleet"));
        is_live = Some(manifest.is_live);
    }

    if honors_operator_flags(opts.site) {
        if opts.no_wake {
            return decision(false, "--no-wake explicit deny");
        }
        if opts.force {
            return decision(true, "--wake explicit force");
        }
    }

    match opts.site {
        AutoWakeSite::Peek => decision(false, "peek never auto-wakes"),
        AutoWakeSite::ApiWake => decision(true, "api-wake endpoint always wakes"),
        AutoWakeSite::Bud => decision(true, "bud always wakes new oracle"),
        AutoWakeSite::WakeCmd => {
            if is_live.unwrap_or(false) {
                decision(false, "wake-cmd: already live (noop)")
            } else {
                decision(true, "wake-cmd: missing — wake")
            }
        }
        AutoWakeSite::View => {
            if is_live.unwrap_or(false) {
                decision(false, "view: target already running")
            } else if is_fleet_known.unwrap_or(false) {
                decision(true, "view: fleet-known and not running")
            } else {
                decision(false, "view: unknown — caller should ask")
            }
        }
        AutoWakeSite::Hey => {
            if opts.is_canonical_target {
                decision(false, "hey: canonical target — skip wake")
            } else if is_live.unwrap_or(false) {
                decision(false, "hey: target already running")
            } else if is_fleet_known.unwrap_or(false) {
                decision(true, "hey: fleet-known and not running")
            } else {
                decision(false, "hey: unknown target — no auto-wake")
            }
        }
        AutoWakeSite::ApiSend => {
            if is_live.unwrap_or(false) {
                decision(false, "api-send: target already running")
            } else if is_fleet_known.unwrap_or(false) {
                decision(true, "api-send: fleet-known and not running")
            } else {
                decision(false, "api-send: unknown target — no auto-wake")
            }
        }
    }
}

fn honors_operator_flags(site: AutoWakeSite) -> bool {
    matches!(
        site,
        AutoWakeSite::View | AutoWakeSite::Hey | AutoWakeSite::ApiSend | AutoWakeSite::WakeCmd
    )
}

fn decision(wake: bool, reason: &str) -> AutoWakeDecision {
    AutoWakeDecision {
        wake,
        reason: reason.to_owned(),
    }
}
