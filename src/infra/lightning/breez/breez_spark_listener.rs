use async_trait::async_trait;
use breez_sdk_spark::{EventListener, PaymentType, SdkEvent};
use tracing::{trace, warn};

use crate::{
    application::entities::Currency,
    domains::event::{EventService, EventUseCases},
};

use super::breez_spark_types::{is_onchain_payment, to_bitcoin_failure_event, to_deposit_event, to_withdrawal_event};

pub(super) struct BreezListener {
    events: EventService,
    currency: Currency,
}

impl BreezListener {
    pub(super) fn new(events: EventService, currency: Currency) -> Self {
        Self { events, currency }
    }
}

#[async_trait]
impl EventListener for BreezListener {
    async fn on_event(&self, e: SdkEvent) {
        match e {
            SdkEvent::PaymentSucceeded { payment } => {
                if is_onchain_payment(&payment.method) {
                    trace!(method = ?payment.method, "PaymentSucceeded (on-chain)");

                    match payment.payment_type {
                        PaymentType::Receive => {
                            if let Some(event) = to_deposit_event(&payment) {
                                let events = self.events.clone();
                                let currency = self.currency.clone();
                                tokio::spawn(async move {
                                    if let Err(err) = events.onchain_deposit(event, currency).await {
                                        warn!(%err, "Failed to process on-chain deposit");
                                    }
                                });
                            }
                        }
                        PaymentType::Send => {
                            if let Some(event) = to_withdrawal_event(&payment) {
                                let events = self.events.clone();
                                tokio::spawn(async move {
                                    if let Err(err) = events.onchain_withdrawal(event).await {
                                        warn!(%err, "Failed to process on-chain withdrawal");
                                    }
                                });
                            }
                        }
                    }
                } else {
                    trace!(method = ?payment.method, "PaymentSucceeded (Lightning/Spark)");

                    match payment.payment_type {
                        PaymentType::Receive => {
                            let events = self.events.clone();
                            tokio::spawn(async move {
                                if let Err(err) = events.invoice_paid(payment.into()).await {
                                    warn!(%err, "Failed to process incoming Lightning/Spark payment");
                                }
                            });
                        }
                        PaymentType::Send => {
                            let events = self.events.clone();
                            tokio::spawn(async move {
                                if let Err(err) = events.outgoing_payment(payment.into()).await {
                                    warn!(%err, "Failed to process outgoing Lightning/Spark payment");
                                }
                            });
                        }
                    }
                }
            }
            SdkEvent::PaymentFailed { payment } => {
                if is_onchain_payment(&payment.method) {
                    trace!("PaymentFailed (on-chain)");

                    if let Some(event) = to_bitcoin_failure_event(&payment) {
                        let events = self.events.clone();
                        tokio::spawn(async move {
                            if let Err(err) = events.failed_payment(event).await {
                                warn!(%err, "Failed to process on-chain payment failure");
                            }
                        });
                    }
                } else {
                    trace!("PaymentFailed (Lightning/Spark)");

                    let events = self.events.clone();
                    tokio::spawn(async move {
                        if let Err(err) = events.failed_payment(payment.into()).await {
                            warn!(%err, "Failed to process Lightning/Spark payment failure");
                        }
                    });
                }
            }
            SdkEvent::PaymentPending { payment } => {
                trace!(
                    method = ?payment.method,
                    is_onchain = is_onchain_payment(&payment.method),
                    "PaymentPending event (payment tracked)"
                );
            }
            SdkEvent::UnclaimedDeposits { .. } => {
                // Spark on-chain deposits must be explicitly claimed (unlike Liquid's automatic swaps).
                // TODO: Consider auto-claiming via sdk.list_unclaimed_deposits + sdk.claim_deposit
                warn!("Unclaimed on-chain deposits detected. Deposits will be auto-claimed on next sync.");
            }
            SdkEvent::ClaimedDeposits { .. } => {
                trace!("On-chain deposits claimed successfully");
            }
            SdkEvent::Synced => {}
            SdkEvent::Optimization { optimization_event } => {
                trace!(?optimization_event, "Spark leaf optimization event");
            }
        }
    }
}
