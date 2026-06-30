use super::ServecoreModuleRegistration;
use crate::serve_core::ServecoreLifecycleModule;
use axum::{response::IntoResponse, routing::get, Extension, Json, Router};
use serde_json::Value;

#[derive(Clone, Copy)]
struct IdentityProvider {
    payload: fn() -> Value,
}

#[must_use]
pub fn identity_lifecycle_module() -> ServecoreLifecycleModule {
    ServecoreLifecycleModule {
        name: "identity".to_owned(),
        weight: 50,
    }
}

#[must_use]
pub fn identity_registration<S>() -> ServecoreModuleRegistration<S>
where
    S: Clone + Send + Sync + 'static,
{
    ServecoreModuleRegistration {
        lifecycle: identity_lifecycle_module(),
        mount: identity_mount,
    }
}

pub fn identity_mount<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    identity_mount_with_provider(
        router,
        IdentityProvider {
            payload: crate::core_impl::serveidentity_http_payload_read_only,
        },
    )
}

fn identity_mount_with_provider<S>(router: Router<S>, provider: IdentityProvider) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/api/identity", get(identity_get))
        .layer(Extension(provider))
}

async fn identity_get(Extension(provider): Extension<IdentityProvider>) -> impl IntoResponse {
    let mut payload = (provider.payload)();
    if let Some(object) = payload.as_object_mut() {
        object.remove("pubkey");
    }
    Json(payload).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve_core::servecore_apply_pipeline;
    use axum::http::StatusCode;
    use std::{net::Ipv4Addr, time::Duration};
    use tokio::sync::oneshot;

    fn identity_fake_payload() -> Value {
        serde_json::from_str(
            r#"{
                "node":"test@local",
                "host":"local",
                "oracle":"gm-bo",
                "version":"1.2.3",
                "agents":["nova"],
                "uptime":42,
                "clockUtc":"2026-06-25T00:00:00.000Z",
                "endpoints":["/api/identity"],
                "pubkey":"pub-test"
            }"#,
        )
        .expect("fake identity payload")
    }

    async fn identity_spawn() -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let router = identity_mount_with_provider(
            Router::new(),
            IdentityProvider {
                payload: identity_fake_payload,
            },
        );
        let app = servecore_apply_pipeline(router);
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                let _ = rx.await;
            });
            server.await.expect("server");
        });
        std::mem::forget(tx);
        addr
    }

    #[test]
    fn identity_lifecycle_matches_public_module_contract() {
        let module = identity_lifecycle_module();
        assert_eq!(module.name, "identity");
        assert_eq!(module.weight, 50);
    }

    #[tokio::test]
    async fn identity_route_is_public_and_returns_redacted_payload() {
        let addr = identity_spawn().await;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("client");
        let response = client
            .get(format!("http://{addr}/api/identity"))
            .send()
            .await
            .expect("identity");
        assert_eq!(response.status(), StatusCode::OK);
        let payload = response.json::<Value>().await.expect("json");
        assert!(
            payload.get("pubkey").is_none(),
            "identity payload must not expose peer_key: {payload}"
        );
        assert!(
            !payload.to_string().contains("pub-test"),
            "identity payload leaked peer_key: {payload}"
        );
        assert_eq!(payload["node"], "test@local");
    }
}
