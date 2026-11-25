use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use swissknife_l402::{ChallengeInvoice, GateError, InvoiceGate, PaidGrant};
use uuid::Uuid;

use crate::application::composition::AppServices;

pub(super) struct Merchant {
    pub services: Arc<AppServices>,
    pub name: String,
    pub wallet_id: Uuid,
    pub price_msat: u64,
    pub requests_per_payment: Option<NonZeroU32>,
    pub invoice_seconds: u32,
    pub challenges: ChallengeLimit,
}

// Shared by every request for one route on this replica. Only new invoices
// consume the allowance, so a paid client can always present its proof.
pub(super) struct ChallengeLimit {
    limit: NonZeroU32,
    window: Mutex<(Instant, u32)>,
}

impl ChallengeLimit {
    pub fn new(limit: NonZeroU32) -> Self {
        Self {
            limit,
            window: Mutex::new((Instant::now(), 0)),
        }
    }

    fn take(&self) -> Result<(), GateError> {
        let mut window = self.window.lock().map_err(|_| GateError::Unavailable)?;
        if window.0.elapsed() >= Duration::from_secs(60) {
            *window = (Instant::now(), 0);
        }
        if window.1 >= self.limit.get() {
            return Err(GateError::Busy);
        }
        window.1 += 1;
        Ok(())
    }
}

#[async_trait]
impl InvoiceGate for Merchant {
    async fn invoice(&self) -> Result<ChallengeInvoice, GateError> {
        self.challenges.take()?;
        let description = match self.requests_per_payment {
            Some(n) => format!("L402 access to {} (up to {n} requests)", self.name),
            None => format!("L402 access to {}", self.name),
        };
        let invoice = self
            .services
            .invoice
            .invoice(
                self.wallet_id,
                self.price_msat,
                Some(description),
                Some(self.invoice_seconds),
            )
            .await
            .map_err(|_| GateError::Unavailable)?;
        let ln = invoice.ln_invoice.ok_or(GateError::Unavailable)?;
        let mut payment_hash = [0; 32];
        hex::decode_to_slice(ln.payment_hash, &mut payment_hash).map_err(|_| GateError::Unavailable)?;
        Ok(ChallengeInvoice {
            payment_hash,
            bolt11: ln.bolt11,
        })
    }

    async fn redeem(&self, grant: &PaidGrant) -> Result<bool, GateError> {
        let Some(limit) = self.requests_per_payment else {
            return Ok(true);
        };
        let used = self
            .services
            .request_meter
            .used(grant.payment_hash)
            .await
            .map_err(|_| GateError::Unavailable)?;
        Ok(used < u64::from(limit.get()))
    }

    async fn record_success(&self, grant: &PaidGrant) -> Result<bool, GateError> {
        let Some(limit) = self.requests_per_payment else {
            return Ok(true);
        };
        self.services
            .request_meter
            .consume(grant.payment_hash, self.wallet_id, &self.name, limit)
            .await
            .map_err(|_| GateError::Unavailable)
    }
}
