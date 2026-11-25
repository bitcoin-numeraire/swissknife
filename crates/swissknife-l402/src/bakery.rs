use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE},
    Engine as _,
};
use bitcoin::hashes::{sha256, Hash};
use l402_proto::L402Token;
use libmacaroon::{Caveat, Format, Macaroon, MacaroonKey, Verifier};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Invalid or expired L402 credential")]
pub struct InvalidCredential;

#[derive(Clone)]
pub struct Bakery {
    root: MacaroonKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaidGrant {
    pub payment_hash: [u8; 32],
    pub token_id: [u8; 32],
}

impl Bakery {
    /// Keep the same independent random root on replicas serving the same paid routes.
    pub fn new(root: &[u8; 32]) -> Self {
        Self {
            root: MacaroonKey::generate(root),
        }
    }

    pub fn mint(&self, scope: &str, payment_hash: [u8; 32], expires_at: u64) -> Result<String, InvalidCredential> {
        let token_id = rand::random::<[u8; 32]>();
        // Aperture's version-zero identifier is a big-endian u16, payment hash and token ID.
        let mut identifier = [0u8; 66];
        identifier[2..34].copy_from_slice(&payment_hash);
        identifier[34..].copy_from_slice(&token_id);
        let mut macaroon = Macaroon::create(None, &self.root, identifier).map_err(|_| InvalidCredential)?;
        macaroon
            .add_first_party_caveat(format!("scope={scope}"))
            .map_err(|_| InvalidCredential)?;
        macaroon
            .add_first_party_caveat(format!("expires={expires_at}"))
            .map_err(|_| InvalidCredential)?;
        let encoded = macaroon.serialize(Format::V2).map_err(|_| InvalidCredential)?;
        Ok(STANDARD.encode(URL_SAFE.decode(encoded).map_err(|_| InvalidCredential)?))
    }

    pub fn verify(&self, scope: &str, header: &str, now: u64) -> Result<PaidGrant, InvalidCredential> {
        self.inspect(scope, header, now)?.ok_or(InvalidCredential)
    }

    // None is a genuine payment for expired or different route terms.
    pub(crate) fn inspect(&self, scope: &str, header: &str, now: u64) -> Result<Option<PaidGrant>, InvalidCredential> {
        if header.len() > 8192 {
            return Err(InvalidCredential);
        }
        let credential = L402Token::from_header(header).map_err(|_| InvalidCredential)?;
        let mut preimage = [0u8; 32];
        hex::decode_to_slice(credential.preimage, &mut preimage).map_err(|_| InvalidCredential)?;
        let payment_hash = sha256::Hash::hash(&preimage).to_byte_array();
        let macaroon = Macaroon::deserialize(credential.macaroon.as_bytes()).map_err(|_| InvalidCredential)?;
        let id = macaroon.identifier();
        if id.len() != 66 || id[..2] != [0, 0] || id[2..34] != payment_hash {
            return Err(InvalidCredential);
        }

        let mut verifier = Verifier::default();
        let mut has_scope = false;
        let mut has_expiry = false;
        let mut applies = true;
        for caveat in macaroon.caveats() {
            let Caveat::FirstParty(caveat) = caveat else {
                return Err(InvalidCredential);
            };
            let predicate = caveat.predicate();
            if let Some(value) = predicate.strip_prefix(b"scope=") {
                has_scope = true;
                applies &= value == scope.as_bytes();
            } else if let Some(value) = predicate.strip_prefix(b"expires=") {
                let expiry = std::str::from_utf8(value)
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .ok_or(InvalidCredential)?;
                has_expiry = true;
                applies &= now < expiry;
            } else {
                return Err(InvalidCredential);
            }
            // Verify the signature over every recognized caveat before deciding
            // whether the authentic credential authorizes or needs renewal.
            verifier.satisfy_exact(predicate);
        }
        if !has_scope || !has_expiry {
            return Err(InvalidCredential);
        }
        verifier
            .verify(&macaroon, &self.root, &[])
            .map_err(|_| InvalidCredential)?;
        if !applies {
            return Ok(None);
        }
        Ok(Some(PaidGrant {
            payment_hash,
            token_id: id[34..].try_into().map_err(|_| InvalidCredential)?,
        }))
    }
}
