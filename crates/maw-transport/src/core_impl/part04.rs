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
