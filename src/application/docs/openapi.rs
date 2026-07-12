use swissknife_types::{ErrorResponse, OrderDirection};

use crate::{
    application::composition::Ledger,
    domains::{
        bitcoin::BtcAddressHandler,
        invoice::InvoiceHandler,
        ln_address::LnAddressHandler,
        lnurl::LnURLHandler,
        nostr::NostrHandler,
        payment::PaymentHandler,
        system::SystemHandler,
        user::{AccountHandler, ApiKeyHandler, AuthHandler},
        wallet::{UserWalletHandler, WalletHandler},
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
        description = "This API is available to anyone with a Numeraire account. The `Me` (`/me`) endpoints expose the authenticated account and wallet-scoped user operations.",
    ),
    components(schemas(OrderDirection, Ledger, ErrorResponse), responses(ErrorResponse)),
    modifiers(&SecurityAddon),
    security(("jwt" = []))
)]
struct ApiDoc;

pub fn merged_openapi() -> OpenApi {
    let mut openapi = ApiDoc::openapi();

    openapi.merge(AuthHandler::openapi());
    openapi.merge(AccountHandler::openapi());
    openapi.merge(UserWalletHandler::openapi());
    openapi.merge(WalletHandler::openapi());
    openapi.merge(InvoiceHandler::openapi());
    openapi.merge(PaymentHandler::openapi());
    openapi.merge(LnAddressHandler::openapi());
    openapi.merge(LnURLHandler::openapi());
    openapi.merge(NostrHandler::openapi());
    openapi.merge(SystemHandler::openapi());
    openapi.merge(ApiKeyHandler::openapi());
    openapi.merge(BtcAddressHandler::openapi());

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
    "reason": "Resource not found"
}
"#;

pub const UNSUPPORTED_EXAMPLE: &str = r#"
{
    "status": "405 Method Not Allowed",
    "reason": "Sign in not allowed (not needed) for oauth2 provider"
}
"#;

pub const CONFLICT_EXAMPLE: &str = r#"
{
    "status": "409 Conflict",
    "reason": "Admin user already created"
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
        components.add_security_scheme("jwt", SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)));
    }
}

#[cfg(test)]
mod openapi_dump {
    use super::merged_openapi;

    /// Regenerates the dashboard's checked-in OpenAPI spec from the live utoipa
    /// annotations. Generation tool, not an assertion: it is `#[ignore]`d so it
    /// does not run during normal `make test`. Invoke on demand via `make openapi`
    /// (or `cargo test dump_openapi_spec -- --ignored --exact`).
    #[test]
    #[ignore = "generation tool; run via `make openapi`"]
    fn dump_openapi_spec() {
        let json = merged_openapi().to_pretty_json().expect("serialize openapi");
        std::fs::write("dashboard/src/lib/openapi.json", json).expect("write openapi.json");
    }
}
