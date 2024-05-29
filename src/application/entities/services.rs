use std::sync::Arc;

use crate::{
    application::dtos::AppConfig,
    domains::{
        invoices::services::{InvoiceService, InvoiceUseCases},
        lightning::services::{
            LnAddressService, LnAddressesUseCases, LnNodeService, LnNodeUseCases,
        },
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
    pub ln_address: Box<dyn LnAddressesUseCases>,
    pub ln_node: Box<dyn LnNodeUseCases>,
}

impl AppServices {
    pub fn new(config: AppConfig, store: AppStore, lightning_client: Arc<dyn LnClient>) -> Self {
        let payments = PaymentService::new(
            store.clone(),
            lightning_client.clone(),
            config.domain.clone(),
            config.fee_buffer.unwrap_or_default(),
        );
        let invoices = InvoiceService::new(
            store.clone(),
            lightning_client.clone(),
            config.invoice_expiry.clone(),
        );
        let ln_address = LnAddressService::new(
            store.clone(),
            lightning_client.clone(),
            config.domain.clone(),
            config.invoice_expiry.clone(),
        );
        let ln_node = LnNodeService::new(lightning_client);
        let wallet = WalletService::new(store);

        AppServices {
            invoice: Box::new(invoices),
            payment: Box::new(payments),
            wallet: Box::new(wallet),
            ln_address: Box::new(ln_address),
            ln_node: Box::new(ln_node),
        }
    }
}
