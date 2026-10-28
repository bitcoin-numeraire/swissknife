use std::time::Duration;

use crate::application::errors::LightningError;
use prost::Message;
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig},
    Request, Status,
};

#[allow(clippy::all, dead_code)]
pub mod api {
    tonic::include_proto!("api");
}
#[allow(clippy::all, dead_code)]
pub mod types {
    tonic::include_proto!("types");
}
#[allow(clippy::all, dead_code)]
pub mod events {
    tonic::include_proto!("events");
}

/// Uses the canonical upstream protobuf and request-binding implementation,
/// with the same Rustls transport as the other SwissKnife gRPC adapters.
pub struct LdkRpc {
    client: api::lightning_node_client::LightningNodeClient<Channel>,
    macaroon: String,
    timeout: Duration,
}

macro_rules! methods {
    ($($method:ident($request:ident) -> $response:ident = $name:literal),+ $(,)?) => {
        $(pub async fn $method(&self, body: api::$request) -> Result<api::$response, Status> {
            let request = self.request($name, body)?;
            Ok(self.client.clone().$method(request).await?.into_inner())
        })+
    }
}

impl LdkRpc {
    pub async fn connect(
        endpoint: &str,
        macaroon: &str,
        certificate: Vec<u8>,
        timeout: Duration,
    ) -> Result<Self, LightningError> {
        ldk_server_macaroons::parse_reusable_macaroon(macaroon).map_err(LightningError::ParseConfig)?;
        if endpoint.contains("://") || endpoint.contains('/') {
            return Err(LightningError::ParseConfig(
                "LDK endpoint must be host:port without a scheme or path".into(),
            ));
        }
        let channel = Channel::from_shared(format!("https://{endpoint}"))
            .map_err(|e| LightningError::ParseConfig(e.to_string()))?
            .connect_timeout(timeout)
            .tls_config(ClientTlsConfig::new().ca_certificate(Certificate::from_pem(certificate)))
            .map_err(|e| LightningError::TLSConfig(e.to_string()))?
            .connect()
            .await
            .map_err(|e| LightningError::Connect(e.to_string()))?;
        Ok(Self {
            client: api::lightning_node_client::LightningNodeClient::new(channel),
            macaroon: macaroon.to_owned(),
            timeout,
        })
    }

    fn request<T: Message>(&self, method: &str, body: T) -> Result<Request<T>, Status> {
        // LDK authenticates the exact uncompressed gRPC frame, including its
        // five-byte prefix. No bearer credential is ever sent directly.
        let encoded = body.encode_to_vec();
        let size = u32::try_from(encoded.len()).map_err(|_| Status::invalid_argument("LDK request too large"))?;
        let mut frame = Vec::with_capacity(encoded.len() + 5);
        frame.push(0);
        frame.extend(size.to_be_bytes());
        frame.extend(encoded);
        let token = ldk_server_macaroons::bind_macaroon_to_request(&self.macaroon, method, &frame)
            .map_err(|_| Status::internal("Failed to bind LDK request credential"))?;
        let mut request = Request::new(body);
        request.metadata_mut().insert(
            "macaroon",
            token
                .parse()
                .map_err(|_| Status::internal("Invalid LDK request credential"))?,
        );
        request.set_timeout(self.timeout);
        Ok(request)
    }

    methods! {
        get_node_info(GetNodeInfoRequest) -> GetNodeInfoResponse = "GetNodeInfo",
        bolt11_receive(Bolt11ReceiveRequest) -> Bolt11ReceiveResponse = "Bolt11Receive",
        bolt11_send(Bolt11SendRequest) -> Bolt11SendResponse = "Bolt11Send",
        list_payments(ListPaymentsRequest) -> ListPaymentsResponse = "ListPayments",
    }
}
