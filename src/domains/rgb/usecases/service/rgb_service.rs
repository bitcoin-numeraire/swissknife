use async_trait::async_trait;

use crate::{adapters::rgb::RGBClient, domains::rgb::usecases::RGBUseCases};

pub struct RGBService {
    pub rgb_client: Box<dyn RGBClient>,
}

impl RGBService {
    pub fn new(rgb_client: Box<dyn RGBClient>) -> Self {
        RGBService { rgb_client }
    }
}

#[async_trait]
impl RGBUseCases for RGBService {}
