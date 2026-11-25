use anyhow::{ensure, Context};
use bitcoin::hashes::{sha256, Hash};
use libmacaroon::{Format, Macaroon};
use reqwest::{header::HeaderValue, Client, StatusCode, Url};
use serde_json::{json, Value};

#[derive(Clone)]
pub(super) struct PassClient {
    http: Client,
    resources: Vec<Url>,
}

impl PassClient {
    pub(super) fn new(resources: &[String]) -> anyhow::Result<Self> {
        let mut urls = Vec::new();
        for resource in resources {
            let url = Url::parse(resource).context("Invalid L402 merchant resource URL")?;
            ensure!(
                super::secure_service_url(&url)
                    && !url.path().ends_with('/')
                    && url.query().is_none()
                    && url.fragment().is_none(),
                "Configure an exact HTTPS resource URL or HTTP on a loopback IP"
            );
            urls.push(url);
        }
        Ok(Self {
            http: super::external_client()?,
            resources: urls,
        })
    }

    pub(super) async fn check(&self, resource: &str, authorization: &str) -> anyhow::Result<Value> {
        let unknown = || json!({"status":"unknown"});
        let url = Url::parse(resource).context("Invalid merchant resource URL")?;
        if !self.resources.contains(&url) {
            return Ok(unknown());
        }
        ensure!(authorization.len() <= 8192, "L402 pass is too large");
        let (scheme, proof) = authorization
            .trim()
            .split_once(char::is_whitespace)
            .context("Expected a cached L402 authorization")?;
        ensure!(
            scheme.eq_ignore_ascii_case("L402") || scheme.eq_ignore_ascii_case("LSAT"),
            "Expected a cached L402 authorization"
        );
        let token = l402_proto::L402Token::from_header(&format!("L402 {}", proof.trim()))
            .map_err(|_| anyhow::anyhow!("Invalid cached L402 authorization"))?;
        let mut macaroon =
            Macaroon::deserialize(token.macaroon.as_bytes()).map_err(|_| anyhow::anyhow!("Invalid cached macaroon"))?;
        let id = macaroon.identifier();
        if id.len() != 66 || id[..2] != [0, 0] {
            return Ok(unknown());
        }
        let mut preimage = [0; 32];
        if hex::decode_to_slice(token.preimage, &mut preimage).is_err()
            || id[2..34] != sha256::Hash::hash(&preimage).to_byte_array()
        {
            return Ok(json!({"status":"invalid"}));
        }
        // Attenuation changes the signature irreversibly. The original pass and preimage stay here.
        macaroon
            .add_first_party_caveat("purpose=status")
            .map_err(|_| anyhow::anyhow!("Could not restrict the pass to inspection"))?;
        let encoded = macaroon
            .serialize(Format::V2)
            .map_err(|_| anyhow::anyhow!("Could not encode the inspection credential"))?;
        let mut inspection = HeaderValue::from_str(&encoded).context("Could not construct the inspection header")?;
        inspection.set_sensitive(true);
        let mut endpoint = url;
        endpoint
            .path_segments_mut()
            .expect("validated HTTP resource")
            .push("status");
        let Ok(response) = self
            .http
            .head(endpoint)
            .header("swissknife-l402-inspection", inspection)
            .send()
            .await
        else {
            return Ok(unknown());
        };
        let state = response
            .headers()
            .get("swissknife-l402-status")
            .and_then(|h| h.to_str().ok());
        let remaining = match response.headers().get("swissknife-l402-remaining") {
            Some(h) => match h.to_str().ok().and_then(|s| s.parse::<u64>().ok()) {
                Some(n) => Some(n),
                None => return Ok(unknown()),
            },
            None => None,
        };
        Ok(match (response.status(), state, remaining) {
            (StatusCode::OK, Some("active"), None | Some(1..)) => {
                json!({"status":"active","remaining_requests":remaining})
            }
            (StatusCode::OK, Some("exhausted"), Some(0)) => json!({"status":"exhausted","remaining_requests":0}),
            (StatusCode::OK, Some("inactive"), None) => json!({"status":"inactive"}),
            (StatusCode::UNAUTHORIZED, Some("invalid"), None) => json!({"status":"invalid"}),
            _ => unknown(),
        })
    }
}
