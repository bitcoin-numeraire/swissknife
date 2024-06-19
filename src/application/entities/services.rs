use std::sync::Arc;

use crate::{
    application::{dtos::AppConfig, errors::ConfigError},
    domains::{
        invoices::services::{InvoiceService, InvoiceUseCases},
        lightning::services::{LnUrlService, LnUrlUseCases},
        payments::services::{PaymentService, PaymentsUseCases},
        users::services::{WalletService, WalletUseCases},
    },
    infra::lightning::{
        breez::BreezClient,
        cln::{ClnGrpcClient, ClnRestClient},
        LnClient,
    },
};

use super::AppStore;

pub struct AppServices {
    pub invoice: Box<dyn InvoiceUseCases>,
    pub payment: Box<dyn PaymentsUseCases>,
    pub wallet: Box<dyn WalletUseCases>,
    pub lnurl: Box<dyn LnUrlUseCases>,
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
            config.invoice_expiry.as_secs() as u32,
        );
        let lnurl = LnUrlService::new(
            store.clone(),
            ln_client.clone(),
            config.invoice_expiry.as_secs() as u32,
            config.domain,
        );
        let wallet = WalletService::new(store);

        AppServices {
            invoice: Box::new(invoices),
            payment: Box::new(payments),
            wallet: Box::new(wallet),
            lnurl: Box::new(lnurl),
        }
    }
}

#[derive(Clone)]
pub enum LnNodeClient {
    Breez(Arc<BreezClient>),
    ClnGrpc(Arc<ClnGrpcClient>),
    ClnRest(Arc<ClnRestClient>),
}

impl LnNodeClient {
    pub fn as_breez_client(&self) -> Result<&BreezClient, ConfigError> {
        if let LnNodeClient::Breez(client) = self {
            Ok(client)
        } else {
            Err(ConfigError::InvalidLightningProvider("Breez".to_string()))
        }
    }

    pub fn as_cln_client(&self) -> Result<&ClnGrpcClient, ConfigError> {
        if let LnNodeClient::ClnGrpc(client) = self {
            Ok(client)
        } else {
            Err(ConfigError::InvalidLightningProvider(
                "CoreLightning".to_string(),
            ))
        }
    }
}
