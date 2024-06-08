use std::sync::Arc;

use crate::{
    application::dtos::AppConfig,
    domains::{
        invoices::services::{InvoiceService, InvoiceUseCases},
        lightning::services::{LnNodeService, LnNodeUseCases, LnUrlService, LnUrlUseCases},
        payments::services::{PaymentService, PaymentsUseCases},
        users::services::{WalletService, WalletUseCases},
    },
    infra::lightning::LnClient,
};

use super::AppStore;

pub struct AppServices {
    pub invoice: Box<dyn InvoiceUseCases>,
    pub payment: Box<dyn PaymentsUseCases>,
    pub wallet: Box<dyn WalletUseCases>,
    pub lnurl: Box<dyn LnUrlUseCases>,
    pub ln_node: Box<dyn LnNodeUseCases>,
}

impl AppServices {
    pub fn new(config: AppConfig, store: AppStore, ln_client: Arc<dyn LnClient>) -> Self {
        let payments = PaymentService::new(
            store.clone(),
            ln_client.clone(),
            config.domain.clone(),
            config.fee_buffer.unwrap_or_default(),
        );
        let invoices = InvoiceService::new(
            store.clone(),
            ln_client.clone(),
            config.invoice_expiry.clone(),
        );
        let lnurl = LnUrlService::new(
            store.clone(),
            ln_client.clone(),
            config.invoice_expiry,
            config.domain,
        );
        let ln_node = LnNodeService::new(ln_client);
        let wallet = WalletService::new(store);

        AppServices {
            invoice: Box::new(invoices),
            payment: Box::new(payments),
            wallet: Box::new(wallet),
            lnurl: Box::new(lnurl),
            ln_node: Box::new(ln_node),
        }
    }
}
