use super::resources::{LegacySubscriptions, ResourceFeeds, WalletResource};
use super::wallet::WalletClient;
use anyhow::{ensure, Context};
use lightning_invoice::Bolt11Invoice;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, InitializeRequestParams, InitializeResult, ListResourcesResult, PaginatedRequestParams,
        ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource, ResourceContents,
        ServerCapabilities, ServerConfig, SubscribeRequestParams, SubscriptionFilter, UnsubscribeRequestParams,
    },
    schemars,
    service::{RequestContext, SubscriptionContext},
    tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct WalletServer {
    wallet: WalletClient,
    tool_router: ToolRouter<Self>,
    feeds: ResourceFeeds,
    legacy: Arc<LegacySubscriptions>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct SaveContact {
    /// Lightning Address to save in the selected wallet, for example billing@example.com.
    ln_address: String,
    /// An optional label of up to 100 printable characters. Omit to clear an existing label.
    label: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct MerchantChallenge {
    /// Full WWW-Authenticate challenge from the merchant's HTTP 402 response.
    challenge: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct PaidChallenge {
    /// Original WWW-Authenticate challenge, paired with the matching wallet payment.
    challenge: String,
    payment_id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct InvoiceDetails {
    /// Signed BOLT11 invoice to inspect.
    invoice: String,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct NewInvoice {
    /// Positive whole millisatoshis as a decimal string.
    amount_msat: String,
    description: Option<String>,
    expiry_seconds: Option<u32>,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct PaymentDetails {
    /// SwissKnife payment UUID.
    id: String,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListPayments {
    /// Between 1 and 100. Defaults to 20.
    limit: Option<u8>,
    offset: Option<u32>,
}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ApprovedPayment {
    /// Destination required by this tool.
    destination: String,
    /// Exact authorized principal in millisatoshis, as a decimal string.
    amount_msat: String,
    /// Maximum authorized routing fee in millisatoshis, as a decimal string.
    maximum_fee_msat: String,
    /// Unique UUID for this approval. Reuse the same UUID and terms after a lost response.
    idempotency_key: String,
}

enum Target {
    Invoice,
    Address,
    Lnurl,
}

fn amount(value: &str) -> anyhow::Result<u64> {
    ensure!(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
        "Amounts must be whole millisatoshis written as decimal strings"
    );
    value.parse().context("Amount exceeds the supported integer range")
}

fn result(value: anyhow::Result<Value>) -> CallToolResult {
    match value {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => CallToolResult::structured_error(json!({"error":error.to_string()})),
    }
}

fn invoice(value: &str) -> anyhow::Result<Bolt11Invoice> {
    value
        .trim()
        .parse::<Bolt11Invoice>()
        .context("Invalid signed BOLT11 invoice")
}

fn describe_invoice(invoice: &Bolt11Invoice) -> Value {
    json!({
        "payment_hash": invoice.payment_hash().to_string(),
        "amount_msat": invoice.amount_milli_satoshis().map(|amount| amount.to_string()),
        "network": invoice.currency().to_string(),
        "description": invoice.description().to_string(),
        "payee": invoice.recover_payee_pub_key().to_string(),
        "expires_at_unix": invoice.expires_at().map(|time| time.as_secs()),
        "expired": invoice.is_expired()
    })
}

fn merchant_invoice(header: &str) -> anyhow::Result<(l402_proto::L402Challenge, Bolt11Invoice)> {
    ensure!(header.len() <= 16384, "Merchant challenge is too large");
    let challenge =
        l402_proto::L402Challenge::from_header(header).map_err(|_| anyhow::anyhow!("Invalid L402 challenge"))?;
    let invoice = invoice(&challenge.invoice)?;
    Ok((challenge, invoice))
}

impl WalletServer {
    pub(super) fn new(wallet: WalletClient, feeds: ResourceFeeds) -> Self {
        Self {
            wallet,
            tool_router: Self::tool_router(),
            feeds,
            legacy: Arc::default(),
        }
    }
    async fn submit(&self, args: ApprovedPayment, kind: Target) -> anyhow::Result<Value> {
        let principal = amount(&args.amount_msat)?;
        let maximum = amount(&args.maximum_fee_msat)?;
        ensure!(
            principal > 0 && principal.checked_add(maximum).is_some(),
            "Invalid principal or fee limit"
        );
        let key = Uuid::parse_str(&args.idempotency_key).context("Invalid idempotency UUID")?;
        let destination = args.destination.trim();
        match kind {
            Target::Invoice => {
                let invoice = invoice(destination)?;
                ensure!(!invoice.is_expired(), "Invoice has expired");
                ensure!(
                    invoice.amount_milli_satoshis().is_none_or(|n| n == principal),
                    "Approved amount differs from the signed invoice"
                );
            }
            Target::Address => {
                let (user, domain) = destination.split_once('@').context("Enter a Lightning Address")?;
                ensure!(
                    !user.is_empty()
                        && !domain.is_empty()
                        && !user.starts_with('₿')
                        && !domain.contains('@')
                        && !destination.contains(['/', ':', '?', '#'])
                        && !destination.chars().any(char::is_whitespace),
                    "Invalid Lightning Address"
                );
            }
            Target::Lnurl => ensure!(
                destination
                    .get(..6)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("lnurl1")),
                "Enter a bech32 LNURL-pay request"
            ),
        }
        self.wallet
            .post(
                "payments/submissions",
                json!({"input":destination,"amount_msat":principal,"maximum_fee_msat":maximum,"idempotency_key":key}),
            )
            .await
    }
}

#[tool_router]
impl WalletServer {
    #[tool(
        description = "Read the selected wallet balance. Amounts are exact decimal millisatoshi strings.",
        annotations(read_only_hint = true)
    )]
    async fn get_balance(&self) -> CallToolResult {
        result(self.wallet.get("balance").await)
    }

    #[tool(
        description = "Read this wallet’s public identity, asset and receiving address.",
        annotations(read_only_hint = true)
    )]
    async fn get_wallet_info(&self) -> CallToolResult {
        result(
            self.wallet.get("").await.map(
                |w| json!({"id":w["id"],"asset":w["asset"],"label":w["label"],"lightning_address":w["ln_address"]}),
            ),
        )
    }

    #[tool(
        description = "Read this wallet’s registered receiving address and whether it is active.",
        annotations(read_only_hint = true)
    )]
    async fn get_lightning_address(&self) -> CallToolResult {
        result(
            self.wallet
                .get("")
                .await
                .map(|w| json!({"lightning_address":w["ln_address"]})),
        )
    }

    #[tool(
        description = "List payment contacts recorded for this wallet.",
        annotations(read_only_hint = true)
    )]
    async fn list_contacts(&self) -> CallToolResult {
        result(self.wallet.get("contacts").await.map(|v| json!({"contacts":v})))
    }

    #[tool(
        description = "List the selected wallet's saved Lightning Addresses, labels and original save times.",
        annotations(read_only_hint = true)
    )]
    async fn list_saved_contacts(&self) -> CallToolResult {
        result(self.wallet.get("saved-contacts").await.map(|v| json!({"contacts":v})))
    }

    #[tool(
        description = "Save a Lightning Address or replace its label in this wallet's address book. Requires manage_contacts permission. Does not initiate a payment.",
        annotations(idempotent_hint = true)
    )]
    async fn add_contact(&self, Parameters(args): Parameters<SaveContact>) -> CallToolResult {
        result(
            self.wallet
                .post(
                    "saved-contacts",
                    json!({"ln_address":args.ln_address,"label":args.label}),
                )
                .await,
        )
    }

    #[tool(
        description = "Decode a signed BOLT11 invoice locally, including amount, expiry, network and payee. No payment is made.",
        annotations(read_only_hint = true)
    )]
    fn decode_invoice(&self, Parameters(args): Parameters<InvoiceDetails>) -> CallToolResult {
        result(invoice(&args.invoice).map(|invoice| describe_invoice(&invoice)))
    }

    #[tool(
        description = "Inspect a merchant's L402 challenge and signed invoice before requesting payment approval. This performs no network access or payment.",
        annotations(read_only_hint = true)
    )]
    fn decode_l402_challenge(&self, Parameters(args): Parameters<MerchantChallenge>) -> CallToolResult {
        result(merchant_invoice(&args.challenge).map(|(challenge, invoice)| {
            let mut value = describe_invoice(&invoice);
            value["invoice"] = json!(challenge.invoice);
            value
        }))
    }

    #[tool(
        description = "Create a private L402 Authorization header from a matching settled payment in this wallet. Only present it to the merchant that issued the challenge. This tool does not send it anywhere or initiate payment.",
        annotations(read_only_hint = true)
    )]
    async fn create_l402_authorization(&self, Parameters(args): Parameters<PaidChallenge>) -> CallToolResult {
        use bitcoin::hashes::{sha256, Hash};
        result(
            async {
                let (challenge, invoice) = merchant_invoice(&args.challenge)?;
                let id = Uuid::parse_str(&args.payment_id).context("Invalid payment UUID")?;
                let payment = self.wallet.get(&format!("payments/{id}")).await?;
                ensure!(payment["status"] == "Settled", "Payment has not settled");
                let lightning = &payment["lightning"];
                let hash = invoice.payment_hash().to_string();
                ensure!(
                    lightning["payment_hash"] == hash,
                    "Payment does not match the challenged invoice"
                );
                let preimage = lightning["payment_preimage"]
                    .as_str()
                    .context("Wallet has no settled invoice preimage")?;
                let mut bytes = [0u8; 32];
                hex::decode_to_slice(preimage, &mut bytes).context("Invalid payment preimage")?;
                ensure!(
                    sha256::Hash::hash(&bytes).to_string() == hash,
                    "Invalid payment preimage"
                );
                let token = l402_proto::L402Token::new(challenge.macaroon, preimage.to_owned());
                Ok(json!({"authorization":token.to_header_value()}))
            }
            .await,
        )
    }

    #[tool(
        description = "Issue a new incoming Lightning invoice. A retry creates another invoice.",
        annotations(destructive_hint = false, idempotent_hint = false)
    )]
    async fn create_invoice(&self, Parameters(args): Parameters<NewInvoice>) -> CallToolResult {
        result(
            async {
                let principal = amount(&args.amount_msat)?;
                ensure!(principal > 0, "Invoice amount must be positive");
                self.wallet
                    .post(
                        "invoices",
                        json!({"amount_msat":principal,"description":args.description,"expiry":args.expiry_seconds}),
                    )
                    .await
            }
            .await,
        )
    }

    #[tool(
        description = "Spend the approved principal to pay a signed BOLT11 invoice. Supply the approved maximum fee and reuse the same idempotency UUID and terms on retry. A submission can still be pending.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn send_payment(&self, Parameters(args): Parameters<ApprovedPayment>) -> CallToolResult {
        result(self.submit(args, Target::Invoice).await)
    }

    #[tool(
        description = "Pay an external Lightning Address with an approved amount, fee cap and stable submission UUID.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn send_to_address(&self, Parameters(args): Parameters<ApprovedPayment>) -> CallToolResult {
        result(self.submit(args, Target::Address).await)
    }

    #[tool(
        description = "Pay a bech32 LNURL-pay request with an approved amount, fee cap and stable submission UUID.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn lnurl_pay(&self, Parameters(args): Parameters<ApprovedPayment>) -> CallToolResult {
        result(self.submit(args, Target::Lnurl).await)
    }

    #[tool(
        description = "List recent outgoing wallet payments with optional pagination.",
        annotations(read_only_hint = true)
    )]
    async fn list_payments(&self, Parameters(args): Parameters<ListPayments>) -> CallToolResult {
        result(
            self.wallet
                .recent(args.limit.unwrap_or(20), args.offset.unwrap_or(0))
                .await,
        )
    }

    #[tool(
        description = "Read one payment and its final or pending status.",
        annotations(read_only_hint = true)
    )]
    async fn get_payment(&self, Parameters(args): Parameters<PaymentDetails>) -> CallToolResult {
        result(
            async {
                let id = Uuid::parse_str(&args.id).context("Invalid payment UUID")?;
                self.wallet.get(&format!("payments/{id}")).await
            }
            .await,
        )
    }

    #[tool(
        description = "Read the execution state of a durable payment submission, including reconciliation and settlement.",
        annotations(read_only_hint = true)
    )]
    async fn get_payment_submission(&self, Parameters(args): Parameters<PaymentDetails>) -> CallToolResult {
        result(
            async {
                let id = Uuid::parse_str(&args.id).context("Invalid submission UUID")?;
                self.wallet.get(&format!("payments/submissions/{id}")).await
            }
            .await,
        )
    }
}

#[tool_handler(router=self.tool_router)]
impl ServerHandler for WalletServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().enable_resources_subscribe().build())
            .with_instructions("This connection selects one wallet. Amounts are exact decimal millisatoshi strings. Obtain approval for the principal and maximum fee before spending. Keep the same payment UUID and terms if a response is lost.")
    }
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        context.peer.set_peer_info(request.clone());
        let mut result = self.negotiate_initialize(&request)?;
        // Legacy HTTP here is stateless. Only stdio keeps a connection for legacy notifications.
        if context.extensions.get::<axum::http::request::Parts>().is_some() {
            if let Some(resources) = &mut result.capabilities.resources {
                resources.subscribe = None;
            }
        }
        Ok(result)
    }

    fn accepted_subscription_filter(&self, requested: &SubscriptionFilter) -> Option<SubscriptionFilter> {
        Some(ResourceFeeds::filter(requested))
    }

    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        self.feeds.listen(&self.wallet, context).await
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        if context.extensions.get::<axum::http::request::Parts>().is_some() {
            return Err(ErrorData::invalid_request(
                "Use subscriptions/listen over Streamable HTTP",
                None,
            ));
        }
        self.legacy
            .subscribe(
                WalletResource::parse(&request.uri)?,
                &self.feeds,
                &self.wallet,
                context.peer,
            )
            .await
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        self.legacy.unsubscribe(WalletResource::parse(&request.uri)?).await;
        Ok(())
    }

    async fn list_resources(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource::new(WalletResource::Balance.uri(), "Wallet balance").with_mime_type("application/json"),
            Resource::new(WalletResource::RecentPayments.uri(), "Recent outgoing payments")
                .with_mime_type("application/json"),
        ]))
    }
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let value = WalletResource::parse(&request.uri)?.read(&self.wallet).await?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(value.to_string(), request.uri).with_mime_type("application/json")
        ])
        .into())
    }
}
