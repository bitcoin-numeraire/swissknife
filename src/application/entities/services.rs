use std::sync::Arc;

use crate::{
    application::dtos::AppConfig,
    domains::{
        invoices::services::{InvoicesService, InvoicesUseCases},
        lightning::services::{LightningService, LightningUseCases},
        payments::services::{PaymentsService, PaymentsUseCases},
        users::services::{WalletService, WalletUseCases},
    },
    infra::lightning::LightningClient,
};

use super::AppStore;

pub struct AppServices {
    pub invoices: Box<dyn InvoicesUseCases>,
    pub payments: Box<dyn PaymentsUseCases>,
    pub wallet: Box<dyn WalletUseCases>,
    pub lightning: Box<dyn LightningUseCases>,
}

impl AppServices {
    pub fn new(
        config: AppConfig,
        store: AppStore,
        lightning_client: Arc<dyn LightningClient>,
    ) -> Self {
        let lightning = LightningService::new(
            store.clone(),
            lightning_client.clone(),
            config.domain.clone(),
            config.invoice_expiry.clone(),
        );
        let payments = PaymentsService::new(
            store.clone(),
            lightning_client.clone(),
            config.domain.clone(),
            config.fee_buffer.unwrap_or_default(),
        );
        let invoices = InvoicesService::new(
            store.clone(),
            lightning_client,
            config.invoice_expiry.clone(),
        );
        let wallet = WalletService::new(store);

        AppServices {
            invoices: Box::new(invoices),
            payments: Box::new(payments),
            wallet: Box::new(wallet),
            lightning: Box::new(lightning),
        }
    }
}
