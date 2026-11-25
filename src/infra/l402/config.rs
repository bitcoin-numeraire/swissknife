use std::{collections::BTreeMap, fmt, num::NonZeroU32};

use reqwest::Url;
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantConfig {
    #[serde(deserialize_with = "read_root_key")]
    pub root_key: [u8; 32],
    pub routes: BTreeMap<String, MerchantRoute>,
}

impl fmt::Debug for MerchantConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MerchantConfig")
            .field("routes", &self.routes.keys())
            .finish_non_exhaustive()
    }
}

fn read_root_key<'de, D: Deserializer<'de>>(input: D) -> Result<[u8; 32], D::Error> {
    let mut root = [0; 32];
    hex::decode_to_slice(String::deserialize(input)?, &mut root)
        .map_err(|_| serde::de::Error::custom("L402 root_key must encode 32 bytes in hex"))?;
    Ok(root)
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantRoute {
    pub wallet_id: Uuid,
    pub target: String,
    pub price_sats: u64,
    pub access_seconds: u32,
    pub requests_per_payment: Option<NonZeroU32>,
    #[serde(default = "default_issue_limit")]
    pub challenges_per_minute: NonZeroU32,
}

fn default_issue_limit() -> NonZeroU32 {
    NonZeroU32::new(30).unwrap_or(NonZeroU32::MIN)
}

impl MerchantRoute {
    pub fn validate(&self, name: &str) -> Result<(Url, u64), &'static str> {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("L402 route names must contain 1–64 lowercase letters, digits or hyphens");
        }
        let url = Url::parse(&self.target).map_err(|_| "invalid L402 target URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err("L402 targets must be HTTP(S) URLs without userinfo or fragments");
        }
        let price = self
            .price_sats
            .checked_mul(1000)
            .filter(|p| (1..=i64::MAX as u64).contains(p))
            .ok_or("L402 price exceeds the wallet range or is zero")?;
        if self.access_seconds < 2 {
            return Err("L402 access_seconds must be at least two seconds");
        }
        Ok((url, price))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn config_rejects_invalid_targets_and_prices_and_redacts_secrets() {
        let mut value = json!({
            "root_key": "ab".repeat(32),
            "routes":{"report":{"wallet_id":Uuid::new_v4(),"price_sats":1,"access_seconds":2,
                "target":"https://merchant.example/report?private=secret"}}
        });
        let config: MerchantConfig = serde_json::from_value(value.clone()).unwrap();
        let route = &config.routes["report"];
        assert_eq!(route.validate("report").unwrap().1, 1000);
        assert!(!format!("{config:?}").contains("ab".repeat(32).as_str()));
        assert!(!format!("{config:?}").contains("secret"));
        assert!(route.validate("bad/{path}").is_err());
        for url in [
            "file:///tmp/report",
            "https://user:pass@example.com/",
            "https://example.com/#fragment",
        ] {
            let mut invalid = route.clone();
            invalid.target = url.into();
            assert!(invalid.validate("report").is_err());
        }
        for price in [0, u64::MAX] {
            let mut invalid = route.clone();
            invalid.price_sats = price;
            assert!(invalid.validate("report").is_err());
        }
        value["root_key"] = json!("a-secret-value");
        let error = serde_json::from_value::<MerchantConfig>(value).unwrap_err().to_string();
        assert!(!error.contains("a-secret-value"));
    }
}
