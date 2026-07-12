use std::sync::Arc;

use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Router,
};
use axum_extra::extract::Query;
use utoipa::OpenApi;
use uuid::Uuid;

use swissknife_types::{
    Account, AccountFilter, CreateAccountRequest, ErrorResponse, UpdateAccountPermissionsRequest, UpdateAccountRequest,
};

use crate::{
    application::{
        composition::AppServices,
        docs::{
            BAD_REQUEST_EXAMPLE, CONFLICT_EXAMPLE, FORBIDDEN_EXAMPLE, INTERNAL_EXAMPLE, NOT_FOUND_EXAMPLE,
            UNAUTHORIZED_EXAMPLE,
        },
        errors::{ApplicationError, DataError},
    },
    infra::axum::{Json, Path},
};

use super::{Permission, User};

#[derive(OpenApi)]
#[openapi(
    paths(
        create_account,
        list_accounts,
        get_account_by_id,
        update_account_by_id,
        replace_account_permissions,
        delete_account_by_id,
        delete_accounts
    ),
    components(schemas(Account, CreateAccountRequest, UpdateAccountRequest, UpdateAccountPermissionsRequest)),
    tags(
        (name = "Accounts", description = "Administrative account management. Requires `read:account` or `write:account` permissions.")
    ),
)]
pub struct AccountHandler;
pub const CONTEXT_PATH: &str = "/v1/accounts";

pub fn router() -> Router<Arc<AppServices>> {
    Router::new()
        .route("/", post(create_account))
        .route("/", get(list_accounts))
        .route("/", delete(delete_accounts))
        .route("/{id}", get(get_account_by_id))
        .route("/{id}", put(update_account_by_id))
        .route("/{id}/permissions", put(replace_account_permissions))
        .route("/{id}", delete(delete_account_by_id))
}

/// Create an account.
#[utoipa::path(
    post,
    path = "",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    request_body = CreateAccountRequest,
    responses(
        (status = 200, description = "Created", body = Account),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn create_account(
    State(services): State<Arc<AppServices>>,
    user: User,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<Json<Account>, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    Ok(Json(services.account.create(payload).await?))
}

/// List accounts.
#[utoipa::path(
    get,
    path = "",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    params(AccountFilter),
    responses(
        (status = 200, description = "Success", body = Vec<Account>),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn list_accounts(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<AccountFilter>,
) -> Result<Json<Vec<Account>>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    Ok(Json(services.account.list(filter).await?))
}

/// Get an account.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Found", body = Account),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn get_account_by_id(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<Json<Account>, ApplicationError> {
    user.check_permission(Permission::ReadAccount)?;
    Ok(Json(services.account.get(id).await?))
}

/// Replace editable account profile fields.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Updated", body = Account),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn update_account_by_id(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
    Json(UpdateAccountRequest { display_name }): Json<UpdateAccountRequest>,
) -> Result<Json<Account>, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    Ok(Json(services.account.update(id, display_name).await?))
}

/// Replace permissions stored for an account.
#[utoipa::path(
    put,
    path = "/{id}/permissions",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    request_body = UpdateAccountPermissionsRequest,
    responses(
        (status = 200, description = "Updated", body = Account),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn replace_account_permissions(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAccountPermissionsRequest>,
) -> Result<Json<Account>, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    Ok(Json(
        services.account.update_permissions(id, payload.permissions).await?,
    ))
}

/// Delete an account and its owned resources.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    responses(
        (status = 200, description = "Deleted"),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 404, description = "Not Found", body = ErrorResponse, example = json!(NOT_FOUND_EXAMPLE)),
        (status = 409, description = "Cannot delete the authenticated account", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_account_by_id(
    State(services): State<Arc<AppServices>>,
    user: User,
    Path(id): Path<Uuid>,
) -> Result<(), ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;
    if id == user.account_id {
        return Err(DataError::Conflict("The authenticated account cannot delete itself.".to_string()).into());
    }

    services.account.delete(id).await
}

/// Delete accounts.
///
/// Deletes the accounts selected by `ids` and returns the number deleted. The
/// authenticated account cannot be included.
#[utoipa::path(
    delete,
    path = "",
    tag = "Accounts",
    context_path = CONTEXT_PATH,
    params(AccountFilter),
    responses(
        (status = 200, description = "Success", body = u64),
        (status = 400, description = "Bad Request", body = ErrorResponse, example = json!(BAD_REQUEST_EXAMPLE)),
        (status = 401, description = "Unauthorized", body = ErrorResponse, example = json!(UNAUTHORIZED_EXAMPLE)),
        (status = 403, description = "Forbidden", body = ErrorResponse, example = json!(FORBIDDEN_EXAMPLE)),
        (status = 409, description = "Cannot delete the authenticated account", body = ErrorResponse, example = json!(CONFLICT_EXAMPLE)),
        (status = 500, description = "Internal Server Error", body = ErrorResponse, example = json!(INTERNAL_EXAMPLE))
    )
)]
async fn delete_accounts(
    State(services): State<Arc<AppServices>>,
    user: User,
    Query(filter): Query<AccountFilter>,
) -> Result<Json<u64>, ApplicationError> {
    user.check_permission(Permission::WriteAccount)?;

    if filter.ids.as_ref().is_none_or(|ids| ids.contains(&user.account_id)) {
        return Err(DataError::Conflict("The authenticated account cannot delete itself.".to_string()).into());
    }

    Ok(services.account.delete_many(filter).await?.into())
}

#[cfg(test)]
mod tests {
    use crate::application::composition::MockAppServicesBuilder;

    use super::*;

    fn user(account_id: Uuid, permissions: Vec<Permission>) -> User {
        User {
            account_id,
            permissions,
        }
    }

    #[tokio::test]
    async fn list_requires_read_account_permission() {
        let services = MockAppServicesBuilder::new().build();

        let result = list_accounts(
            State(Arc::new(services)),
            user(Uuid::new_v4(), vec![]),
            Query(AccountFilter::default()),
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::Authorization(_))));
    }

    #[tokio::test]
    async fn update_permissions_requires_write_account_permission() {
        let services = MockAppServicesBuilder::new().build();

        let result = replace_account_permissions(
            State(Arc::new(services)),
            user(Uuid::new_v4(), vec![]),
            Path(Uuid::new_v4()),
            Json(UpdateAccountPermissionsRequest {
                permissions: vec![Permission::ReadWallet],
            }),
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::Authorization(_))));
    }

    #[tokio::test]
    async fn delete_rejects_the_authenticated_account() {
        let account_id = Uuid::new_v4();
        let services = MockAppServicesBuilder::new().build();

        let result = delete_account_by_id(
            State(Arc::new(services)),
            user(account_id, vec![Permission::WriteAccount]),
            Path(account_id),
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::Data(DataError::Conflict(_)))));
    }

    #[tokio::test]
    async fn delete_many_rejects_the_authenticated_account() {
        let account_id = Uuid::new_v4();
        let services = MockAppServicesBuilder::new().build();

        let result = delete_accounts(
            State(Arc::new(services)),
            user(account_id, vec![Permission::WriteAccount]),
            Query(AccountFilter {
                ids: Some(vec![account_id]),
                ..Default::default()
            }),
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::Data(DataError::Conflict(_)))));
    }
}
