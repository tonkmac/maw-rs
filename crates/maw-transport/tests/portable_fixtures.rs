use maw_transport::{
    classify_error, ClassifiedError, Transport, TransportFailureReason, TransportResult,
    TransportRouter, TransportTarget,
};
use serde::Deserialize;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Fixture {
    #[serde(rename = "classifyError")]
    ClassifyError {
        name: String,
        error: Option<String>,
        expected: ExpectedClassifiedError,
    },
    Send {
        name: String,
        target: Option<FixtureTarget>,
        message: Option<String>,
        from: Option<String>,
        transports: Vec<FixtureTransport>,
        expected: ExpectedSend,
    },
}

#[derive(Debug, Deserialize)]
struct FixtureTarget {
    oracle: String,
    host: Option<String>,
    #[serde(rename = "tmuxTarget")]
    tmux_target: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureTransport {
    name: String,
    connected: Option<bool>,
    can_reach: Option<bool>,
    send: Option<SendAction>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SendAction {
    Ok,
    False,
    Throw { error: String },
}

#[derive(Debug, Deserialize)]
struct ExpectedClassifiedError {
    reason: String,
    retryable: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedSend {
    result: ExpectedTransportResult,
    sent: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTransportResult {
    ok: bool,
    via: String,
    reason: Option<String>,
    retryable: bool,
}

struct FixtureTransportRuntime {
    fixture: FixtureTransport,
    sent: Rc<RefCell<Vec<String>>>,
}

impl Transport for FixtureTransportRuntime {
    fn name(&self) -> &str {
        &self.fixture.name
    }

    fn connected(&self) -> bool {
        self.fixture.connected.unwrap_or(true)
    }

    fn can_reach(&self, _target: &TransportTarget) -> bool {
        self.fixture.can_reach.unwrap_or(true)
    }

    fn send(
        &mut self,
        _target: &TransportTarget,
        _message: &str,
        _from: &str,
    ) -> Result<bool, String> {
        self.sent.borrow_mut().push(self.fixture.name.clone());
        match self.fixture.send.as_ref().unwrap_or(&SendAction::Ok) {
            SendAction::Ok => Ok(true),
            SendAction::False => Ok(false),
            SendAction::Throw { error } => Err(error.clone()),
        }
    }
}

impl From<FixtureTarget> for TransportTarget {
    fn from(target: FixtureTarget) -> Self {
        Self {
            oracle: target.oracle,
            host: target.host,
            tmux_target: target.tmux_target,
        }
    }
}

fn reason_from_str(reason: &str) -> TransportFailureReason {
    match reason {
        "timeout" => TransportFailureReason::Timeout,
        "unreachable" => TransportFailureReason::Unreachable,
        "auth" => TransportFailureReason::Auth,
        "rate_limit" => TransportFailureReason::RateLimit,
        "rejected" => TransportFailureReason::Rejected,
        "parse_error" => TransportFailureReason::ParseError,
        "unknown" => TransportFailureReason::Unknown,
        other => panic!("unknown fixture reason: {other}"),
    }
}

fn expected_classified(expected: &ExpectedClassifiedError) -> ClassifiedError {
    ClassifiedError {
        reason: reason_from_str(&expected.reason),
        retryable: expected.retryable,
    }
}

fn expected_result(expected: ExpectedTransportResult) -> TransportResult {
    TransportResult {
        ok: expected.ok,
        via: expected.via,
        reason: expected.reason.as_deref().map(reason_from_str),
        retryable: expected.retryable,
    }
}

#[test]
fn transport_router_fixtures_match_maw_js_portable_spec() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("fixtures/transport-router.fixtures.json"))
            .expect("valid transport router fixture json");

    for fixture in fixtures {
        match fixture {
            Fixture::ClassifyError {
                name,
                error,
                expected,
            } => {
                assert_eq!(
                    classify_error(error.as_deref()),
                    expected_classified(&expected),
                    "{name}"
                );
            }
            Fixture::Send {
                name,
                target,
                message,
                from,
                transports,
                expected,
            } => {
                let sent = Rc::new(RefCell::new(Vec::new()));
                let mut router = TransportRouter::new();
                for transport in transports {
                    router.register(FixtureTransportRuntime {
                        fixture: transport,
                        sent: Rc::clone(&sent),
                    });
                }
                let target = target.map_or_else(
                    || TransportTarget {
                        oracle: "neo".to_owned(),
                        host: None,
                        tmux_target: Some("neo:1".to_owned()),
                    },
                    Into::into,
                );
                let result = router.send(
                    &target,
                    message.as_deref().unwrap_or("hello"),
                    from.as_deref().unwrap_or("codex"),
                );
                assert_eq!(result, expected_result(expected.result), "{name}");
                assert_eq!(*sent.borrow(), expected.sent, "sent order: {name}");
            }
        }
    }
}
