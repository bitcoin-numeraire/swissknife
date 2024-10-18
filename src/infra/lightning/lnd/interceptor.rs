use std::str::FromStr;

use tonic::{metadata::AsciiMetadataValue, service::Interceptor, Request, Status};

use crate::application::errors::LightningError;

pub(crate) struct MacaroonInterceptor {
    macaroon: AsciiMetadataValue,
}

impl MacaroonInterceptor {
    pub(crate) fn new(macaroon_hex: &str) -> Result<Self, LightningError> {
        let macaroon = AsciiMetadataValue::from_str(&macaroon_hex)
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        Ok(Self { macaroon })
    }
}

impl Interceptor for MacaroonInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        req.metadata_mut().insert("macaroon", self.macaroon.clone());
        Ok(req)
    }
}
