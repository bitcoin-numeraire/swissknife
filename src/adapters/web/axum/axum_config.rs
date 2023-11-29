use std::{fs::File, io::BufReader, sync::Arc};

use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

pub struct AxumServerConfig {
    pub addr: String,
    pub tls_key_path: Option<String>,
    pub tls_cert_path: Option<String>,
}

impl AxumServerConfig {
    pub fn tls_config(&self) -> Option<Arc<ServerConfig>> {
        let tls_key_path = self.tls_key_path.as_ref()?;
        let tls_cert_path = self.tls_cert_path.as_ref()?;

        let mut key_reader = BufReader::new(File::open(tls_key_path).ok()?);
        let mut cert_reader = BufReader::new(File::open(tls_cert_path).ok()?);

        let key = PrivateKey(pkcs8_private_keys(&mut key_reader).unwrap().remove(0));
        let certs = certs(&mut cert_reader)
            .ok()?
            .into_iter()
            .map(Certificate)
            .collect();

        let mut config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .ok()?;

        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Some(Arc::new(config))
    }
}
