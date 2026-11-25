use std::{net::SocketAddr, sync::Arc};

use anyhow::{ensure, Context};
use axum::http::HeaderValue;
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use reqwest::Url;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use subtle::ConstantTimeEq;

use crate::{tools::WalletServer, wallet::WalletClient};

#[derive(Clone)]
struct Access([u8; 32]);

impl Access {
    fn parse(secret: &str) -> anyhow::Result<Self> {
        let mut raw = [0; 32];
        hex::decode_to_slice(secret, &mut raw).context("MCP HTTP token must encode 32 random bytes in hex")?;
        Ok(Self(raw))
    }
}

async fn authenticate(State(expected): State<Access>, mut request: Request, next: Next) -> Response {
    let valid = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .and_then(|(_, token)| Access::parse(token).ok())
        .is_some_and(|token| bool::from(expected.0.ct_eq(&token.0)));
    if !valid {
        return (
            StatusCode::UNAUTHORIZED,
            [
                (header::WWW_AUTHENTICATE, "Bearer realm=\"SwissKnife MCP\""),
                (header::CACHE_CONTROL, "no-store"),
            ],
        )
            .into_response();
    }
    request.headers_mut().remove(header::AUTHORIZATION);
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn allowed_origins(local: SocketAddr, public_origin: Option<&str>) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    let mut hosts = vec![local.to_string(), format!("localhost:{}", local.port())];
    let mut origins: Vec<String> = hosts.iter().map(|host| format!("http://{host}")).collect();
    if let Some(origin) = public_origin {
        let url = Url::parse(origin).context("Invalid public MCP origin")?;
        ensure!(
            url.scheme() == "https"
                && url.host().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.path() == "/"
                && url.query().is_none()
                && url.fragment().is_none(),
            "Supply an HTTPS origin for the TLS proxy"
        );
        let authority = url.origin().ascii_serialization();
        hosts.push(authority.trim_start_matches("https://").to_owned());
        // The SDK requires an explicit port to restrict a browser origin to its default HTTPS port.
        let host = url.host_str().context("Public MCP origin requires a hostname")?;
        origins.push(format!("https://{host}:{}", url.port().unwrap_or(443)));
    }
    Ok((hosts, origins))
}

pub(super) async fn serve(wallet: WalletClient, listen: SocketAddr, public_origin: Option<&str>) -> anyhow::Result<()> {
    ensure!(
        listen.ip().is_loopback(),
        "MCP HTTP must bind to loopback; expose it through a TLS proxy or private tunnel"
    );
    let secret = std::env::var("SWISSKNIFE_MCP_HTTP_TOKEN")
        .context("Set a separate SWISSKNIFE_MCP_HTTP_TOKEN for remote clients")?;
    let access = Access::parse(&secret)?;
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .context("Cannot bind MCP HTTP listener")?;
    let local = listener.local_addr()?;
    let (hosts, origins) = allowed_origins(local, public_origin)?;
    let config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(hosts)
        .with_allowed_origins(origins)
        .with_legacy_session_mode(false)
        .with_max_request_body_bytes(65536);
    let feeds = crate::resources::ResourceFeeds::default();
    let service = StreamableHttpService::new(
        move || Ok(WalletServer::new(wallet.clone(), feeds.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let router = Router::new()
        .route_service("/mcp", service)
        .route_layer(middleware::from_fn_with_state(access, authenticate));
    eprintln!("Serving wallet MCP on http://{local}/mcp");
    axum::serve(listener, router).await?;
    Ok(())
}
