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

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn rate_limit_like(msg: &str) -> bool {
    msg.contains("rate") && msg.contains("limit")
}
