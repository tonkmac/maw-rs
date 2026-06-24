use std::time::Duration;

use maw_auth::sign_headers_v3_at;
use serde::Deserialize;

const SEND_PATH: &str = "/api/send";
const SEND_METHOD: &str = "POST";

/// Outbound `/api/send` request, signed with maw v3 from-signing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSendRequest {
    pub peer_url: String,
    pub target: String,
    pub text: String,
    pub inbox: Option<bool>,
    pub from: String,
    pub peer_key: String,
    pub timestamp: i64,
}

/// Parsed `/api/send` response outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSendResponse {
    pub ok: bool,
    pub status: u16,
    pub state: Option<String>,
    pub target: Option<String>,
    pub last_line: Option<String>,
    pub error: Option<String>,
}

impl PeerSendResponse {
    #[must_use]
    pub fn delivered_or_queued(&self) -> bool {
        self.ok
            && matches!(
                self.state.as_deref().unwrap_or("queued"),
                "delivered" | "queued"
            )
    }
}

/// Concrete reqwest/rustls HTTP adapter for maw federation endpoints.
#[derive(Clone)]
pub struct ReqwestHttpTransportIo {
    client: reqwest::Client,
    timeout_ms: u64,
}

impl ReqwestHttpTransportIo {
    /// Build a reqwest client with rustls-only TLS features.
    ///
    /// # Errors
    ///
    /// Returns reqwest builder errors.
    pub fn new(timeout_ms: u64) -> Result<Self, String> {
        let timeout = Duration::from_millis(timeout_ms);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| format!("http client build failed: {error}"))?;
        Ok(Self { client, timeout_ms })
    }

    #[must_use]
    pub const fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// POST a signed maw v3 `/api/send` request.
    ///
    /// # Errors
    ///
    /// Returns a clear transport/auth/parse error string on failure.
    pub async fn send_peer(&self, request: &PeerSendRequest) -> Result<PeerSendResponse, String> {
        let body = peer_send_body(&request.target, &request.text, request.inbox)?;
        let headers = sign_headers_v3_at(
            &request.peer_key,
            &request.from,
            SEND_METHOD,
            SEND_PATH,
            Some(body.as_bytes()),
            request.timestamp,
        )?;
        let url = format!("{}{}", request.peer_url.trim_end_matches('/'), SEND_PATH);
        let mut builder = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        for (name, value) in headers.to_btree_map() {
            builder = builder.header(name.as_str(), value.as_str());
        }

        let response = builder
            .send()
            .await
            .map_err(|error| format!("network error posting {url}: {error}"))?;
        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|error| format!("network error reading {url}: {error}"))?;
        let wire = serde_json::from_str::<PeerSendWireResponse>(&text)
            .map_err(|error| format!("failed to parse /api/send response: {error}; body={text}"))?;
        let parsed = PeerSendResponse {
            ok: wire.ok.unwrap_or(false),
            status,
            state: wire.state,
            target: wire.target,
            last_line: wire.last_line,
            error: wire.error,
        };
        if status >= 400 {
            return Err(format!(
                "remote /api/send returned HTTP {status}: {}",
                parsed.error.as_deref().unwrap_or("request failed")
            ));
        }
        if !parsed.delivered_or_queued() {
            return Err(format!(
                "remote /api/send failed: state={} error={}",
                parsed.state.as_deref().unwrap_or("-"),
                parsed.error.as_deref().unwrap_or("remote returned ok=false")
            ));
        }
        Ok(parsed)
    }
}

impl HttpTransportIo for ReqwestHttpTransportIo {
    fn list_local_sessions(&mut self) -> Result<Vec<TmuxTransportSession>, String> {
        Ok(Vec::new())
    }

    fn get_all_sessions(
        &mut self,
        _local_sessions: &[TmuxTransportSession],
    ) -> Result<Vec<TransportSession>, String> {
        Ok(Vec::new())
    }

    fn find_target_window(&mut self, _sessions: &[TransportSession], _query: &str) -> Option<String> {
        None
    }

    fn send_peer_keys(
        &mut self,
        _source: &str,
        _target: &str,
        _message: &str,
    ) -> Result<bool, String> {
        Err("sync send_peer_keys is not supported by the async reqwest transport".to_owned())
    }

    fn post_peer_feed(
        &mut self,
        _url: &str,
        _method: &str,
        _body: &str,
        _timeout_ms: u64,
    ) -> Result<HttpPostResult, String> {
        Err("sync post_peer_feed is not supported by the async reqwest transport".to_owned())
    }

    fn timeout_for(&self, _transport: &str) -> u64 {
        self.timeout_ms
    }
}

#[derive(Debug, Deserialize)]
struct PeerSendWireResponse {
    #[serde(default)]
    ok: Option<bool>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default, rename = "lastLine")]
    last_line: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Build the exact v26.6.13 `/api/send` JSON body: target, text, and optional inbox.
///
/// # Errors
///
/// Returns JSON serialization errors for non-representable strings.
pub fn peer_send_body(target: &str, text: &str, inbox: Option<bool>) -> Result<String, String> {
    let target = serde_json::to_string(target).map_err(|error| error.to_string())?;
    let text = serde_json::to_string(text).map_err(|error| error.to_string())?;
    Ok(match inbox {
        Some(inbox) => format!(r#"{{"target":{target},"text":{text},"inbox":{inbox}}}"#),
        None => format!(r#"{{"target":{target},"text":{text}}}"#),
    })
}

#[cfg(test)]
mod tests {
    use super::peer_send_body;

    #[test]
    fn peer_send_body_keeps_wire_field_order_and_optional_inbox() {
        assert_eq!(
            peer_send_body("remote-oracle", "E1 signed capture", Some(true)).unwrap(),
            r#"{"target":"remote-oracle","text":"E1 signed capture","inbox":true}"#
        );
        assert_eq!(
            peer_send_body("remote-oracle", "hello", None).unwrap(),
            r#"{"target":"remote-oracle","text":"hello"}"#
        );
    }
}
