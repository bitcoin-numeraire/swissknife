use std::sync::Arc;

use rustls::{
    client::{
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        WebPkiServerVerifier,
    },
    pki_types::{CertificateDer, ServerName, UnixTime},
    DigitallySignedStruct, Error, RootCertStore, SignatureScheme,
};

use crate::application::errors::LightningError;

#[derive(Debug)]
pub(crate) struct AllowCaAsEndEntityVerifier {
    inner: Arc<WebPkiServerVerifier>,
}

impl AllowCaAsEndEntityVerifier {
    pub(crate) fn new(tls_cert: Vec<u8>) -> Result<Self, LightningError> {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from_slice(tls_cert.as_slice()))
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        let verifier = WebPkiServerVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?;

        Ok(Self { inner: verifier })
    }
}

impl ServerCertVerifier for AllowCaAsEndEntityVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer,
        _intermediates: &[CertificateDer],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        match self.inner.verify_server_cert(
            end_entity,
            _intermediates,
            _server_name,
            _ocsp_response,
            _now,
        ) {
            Err(Error::Other(reason)) if reason.to_string().contains("CaUsedAsEndEntity") => {
                println!("Allowing CA as end entity");
                Ok(ServerCertVerified::assertion())
            }
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}
