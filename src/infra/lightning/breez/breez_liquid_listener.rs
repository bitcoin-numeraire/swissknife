use breez_sdk_liquid::prelude::{EventListener, PaymentType, SdkEvent};
use tracing::{trace, warn};

use crate::{
    application::entities::Currency,
    domains::event::{EventService, EventUseCases},
};

use super::breez_liquid_types::{is_bitcoin_payment, to_bitcoin_failure_event, to_deposit_event, to_withdrawal_event};

pub(super) struct BreezListener {
    events: EventService,
    currency: Currency,
}

impl BreezListener {
    pub(super) fn new(events: EventService, currency: Currency) -> Self {
        Self { events, currency }
    }
}

impl EventListener for BreezListener {
    fn on_event(&self, e: SdkEvent) {
        match e {
            SdkEvent::PaymentSucceeded { details } => {
                if is_bitcoin_payment(&details.details) {
                    trace!(payment_type = ?details.payment_type, "PaymentSucceeded (Bitcoin chain swap)");

                    match details.payment_type {
                        PaymentType::Receive => {
                            if let Some(event) = to_deposit_event(&details) {
                                let events = self.events.clone();
                                let currency = self.currency.clone();
                                tokio::spawn(async move {
                                    if let Err(err) = events.onchain_deposit(event, currency).await {
                                        warn!(%err, "Failed to process Bitcoin onchain deposit");
                                    }
                                });
                            }
                        }
                        PaymentType::Send => {
                            if let Some(event) = to_withdrawal_event(&details) {
                                let events = self.events.clone();
                                tokio::spawn(async move {
                                    if let Err(err) = events.onchain_withdrawal(event).await {
                                        warn!(%err, "Failed to process Bitcoin onchain withdrawal");
                                    }
                                });
                            }
                        }
                    }
                } else {
                    trace!(payment_type = ?details.payment_type, "PaymentSucceeded (Lightning)");

                    match details.payment_type {
                        PaymentType::Receive => {
                            let events = self.events.clone();
                            tokio::spawn(async move {
                                if let Err(err) = events.invoice_paid(details.into()).await {
                                    warn!(%err, "Failed to process incoming Lightning payment");
                                }
                            });
                        }
                        PaymentType::Send => {
                            let events = self.events.clone();
                            tokio::spawn(async move {
                                if let Err(err) = events.outgoing_payment(details.into()).await {
                                    warn!(%err, "Failed to process outgoing Lightning payment");
                                }
                            });
                        }
                    }
                }
            }
            SdkEvent::PaymentFailed { details } => {
                if is_bitcoin_payment(&details.details) {
                    trace!("PaymentFailed (Bitcoin chain swap)");

                    if let Some(event) = to_bitcoin_failure_event(&details) {
                        let events = self.events.clone();
                        tokio::spawn(async move {
                            if let Err(err) = events.failed_payment(event).await {
                                warn!(%err, "Failed to process Bitcoin chain swap failure");
                            }
                        });
                    }
                } else {
                    trace!("PaymentFailed (Lightning)");

                    let events = self.events.clone();
                    tokio::spawn(async move {
                        if let Err(err) = events.failed_payment(details.into()).await {
                            warn!(%err, "Failed to process Lightning payment failure");
                        }
                    });
                }
            }
            SdkEvent::PaymentPending { details } => {
                trace!(
                    payment_type = ?details.payment_type,
                    is_bitcoin = is_bitcoin_payment(&details.details),
                    "PaymentPending event (no action needed, payment already tracked)"
                );
            }
            SdkEvent::PaymentWaitingConfirmation { details } => {
                trace!(
                    payment_type = ?details.payment_type,
                    is_bitcoin = is_bitcoin_payment(&details.details),
                    "PaymentWaitingConfirmation event"
                );
            }
            SdkEvent::PaymentWaitingFeeAcceptance { details } => {
                trace!(
                    payment_type = ?details.payment_type,
                    "PaymentWaitingFeeAcceptance event"
                );
            }
            SdkEvent::PaymentRefundable { details } => {
                trace!(payment_type = ?details.payment_type, "PaymentRefundable event (not handled)");
            }
            SdkEvent::PaymentRefundPending { details } => {
                trace!(payment_type = ?details.payment_type, "PaymentRefundPending event (not handled)");
            }
            SdkEvent::PaymentRefunded { details } => {
                trace!(payment_type = ?details.payment_type, "PaymentRefunded event (not handled)");
            }
            SdkEvent::Synced => {
                trace!("Breez SDK synced");
            }
            SdkEvent::SyncFailed { error } => {
                warn!(%error, "Breez SDK sync failed");
            }
            SdkEvent::DataSynced { did_pull_new_records } => {
                trace!(did_pull_new_records, "Breez SDK data synced");
            }
        }
    }
}
