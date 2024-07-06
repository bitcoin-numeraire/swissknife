use crate::{
    application::{
        dtos::ErrorResponse,
        entities::{Currency, Ledger, OrderDirection},
    },
    domains::{
        invoices::api::InvoiceHandler,
        lightning::api::{BreezNodeHandler, LnAddressHandler, LnURLpHandler},
        payments::api::PaymentHandler,
        system::api::SystemHandler,
        users::api::AuthHandler,
        wallet::api::WalletHandler,
    },
};
use utoipa::{
    openapi::{
        security::{Http, HttpAuthScheme, SecurityScheme},
        Components, OpenApi,
    },
    Modify, OpenApi as OpenApiDoc,
};

#[derive(OpenApiDoc)]
#[openapi(
    info(
        title = "Numeraire SwissKnife REST API",
        description = "This API is available to anyone with a Numeraire account",
    ),
    components(schemas(OrderDirection, Ledger, Currency), responses(ErrorResponse)),
    modifiers(&SecurityAddon),
)]
struct ApiDoc;

pub fn merged_openapi() -> OpenApi {
    let mut openapi = ApiDoc::openapi();
    openapi.merge(AuthHandler::openapi());
    openapi.merge(WalletHandler::openapi());
    openapi.merge(InvoiceHandler::openapi());
    openapi.merge(PaymentHandler::openapi());
    openapi.merge(LnAddressHandler::openapi());
    openapi.merge(LnURLpHandler::openapi());
    openapi.merge(BreezNodeHandler::openapi());
    openapi.merge(SystemHandler::openapi());
    openapi
}

pub const BAD_REQUEST_EXAMPLE: &str = r#"
{
    "status": "400 Bad Request",
    "reason": "Missing required parameter in request"
}
"#;

pub const UNAUTHORIZED_EXAMPLE: &str = r#"
{
    "status": "401 Unauthorized",
    "reason": "Invalid credentials"
}
"#;

pub const FORBIDDEN_EXAMPLE: &str = r#"
{
    "status": "403 Forbidden",
    "reason": "Missing permissions"
}
"#;

pub const NOT_FOUND_EXAMPLE: &str = r#"
{
    "status": "404 Not Found",
    "reason": "Resouce not found"
}
"#;

pub const UNSUPPORTED_EXAMPLE: &str = r#"
{
    "status": "405 Method Not Allowed",
    "reason": "Sign in not allowed (not needed) for oauth2 provider"
}
"#;

pub const UNPROCESSABLE_EXAMPLE: &str = r#"
{
    "status": "422 Unprocessable Entity",
    "reason": "Validation failed: ..."
}
"#;

pub const INTERNAL_EXAMPLE: &str = r#"
{
    "status": "500 Internal Server Error",
    "reason": "Internal server error, Please contact your administrator or try later"
}
"#;

struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut OpenApi) {
        let components: &mut Components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
        components.add_security_scheme(
            "jwt",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}
