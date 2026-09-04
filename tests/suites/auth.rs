//! `/v1/auth/*` (local JWT) plus auth-middleware enforcement on protected routes.

use futures_util::future::join_all;
use reqwest::StatusCode;

use swissknife_types::{ChangePasswordRequest, SignInRequest, SignInResponse, SignUpRequest};

use crate::common::client::ApiClient;
use crate::common::harness::{matrix_cell, spawn_instance, ADMIN_PASSWORD};
use crate::common::{app, assert_error, assert_status, Auth};

mod sign_in {
    use super::*;

    #[tokio::test]
    async fn with_the_correct_password_returns_a_token() {
        let app = app().await;
        app.admin_token().await; // ensure the admin exists
        let res = app
            .api()
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    username: "admin".into(),
                    password: ADMIN_PASSWORD.to_string(),
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        assert!(!res.parse::<SignInResponse>().token.is_empty(), "{}", res.body);
    }

    #[tokio::test]
    async fn with_a_wrong_password_is_unauthorized() {
        let app = app().await;
        app.admin_token().await;
        let res = app
            .api()
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    username: "admin".into(),
                    password: "wrong-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }
}

mod sign_up {
    use super::*;

    #[tokio::test]
    async fn a_second_sign_up_conflicts() {
        let app = app().await;
        app.admin_token().await; // first admin already created
        let res = app
            .api()
            .post(
                "/v1/auth/sign-up",
                Auth::None,
                SignUpRequest {
                    password: "another-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn concurrent_sign_ups_have_one_winner_and_no_server_errors() {
        let (database, provider) = matrix_cell();
        let label = format!("{database}-{provider}-auth-concurrent-sign-up");
        let spawned = spawn_instance(&database, &provider, &label, &[]).await;
        let api = ApiClient::new(spawned.base_url);

        let responses = join_all((0..8).map(|_| {
            let api = api.clone();
            async move {
                api.post(
                    "/v1/auth/sign-up",
                    Auth::None,
                    SignUpRequest {
                        password: ADMIN_PASSWORD.to_string(),
                    },
                )
                .await
            }
        }))
        .await;

        let mut winner = None;
        let mut conflicts = 0;
        for response in responses {
            match response.status {
                StatusCode::OK => winner = Some(response.parse::<SignInResponse>().token),
                StatusCode::CONFLICT => conflicts += 1,
                status => panic!("concurrent sign-up returned {status}: {}", response.body),
            }
        }
        assert!(winner.is_some(), "one sign-up must create the admin account");
        assert_eq!(conflicts, 7, "all losing sign-ups must report conflict");

        let token = winner.expect("sign-up winner token");
        let profile = api.get("/v1/me", Auth::Bearer(&token)).await;
        assert_status(&profile, StatusCode::OK);
        let wallets = api.get("/v1/me/wallets", Auth::Bearer(&token)).await;
        assert_status(&wallets, StatusCode::OK);
        assert_eq!(wallets.parse::<Vec<swissknife_types::Wallet>>().len(), 1);
    }
}

mod change_password {
    use super::*;

    #[tokio::test]
    async fn rejects_missing_credentials() {
        let app = app().await;
        let res = app
            .api()
            .post(
                "/v1/auth/change-password",
                Auth::None,
                ChangePasswordRequest {
                    current_password: ADMIN_PASSWORD.to_string(),
                    new_password: "new-integration-admin-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn with_a_wrong_current_password_is_unprocessable() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app
            .api()
            .post(
                "/v1/auth/change-password",
                Auth::Bearer(token),
                ChangePasswordRequest {
                    current_password: "wrong-password".to_string(),
                    new_password: "new-integration-admin-password".to_string(),
                },
            )
            .await;
        assert_error(&res, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn with_the_correct_current_password_updates_future_sign_ins() {
        let api = isolated_api("auth-change-password").await;
        let admin_token = bootstrap_admin(&api, ADMIN_PASSWORD).await;
        let new_password = "new-integration-admin-password";

        let res = api
            .post(
                "/v1/auth/change-password",
                Auth::Bearer(&admin_token),
                ChangePasswordRequest {
                    current_password: ADMIN_PASSWORD.to_string(),
                    new_password: new_password.to_string(),
                },
            )
            .await;
        assert_status(&res, StatusCode::NO_CONTENT);

        let old_password = api
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    username: "admin".into(),
                    password: ADMIN_PASSWORD.to_string(),
                },
            )
            .await;
        assert_error(&old_password, StatusCode::UNAUTHORIZED);

        let new_password = api
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                SignInRequest {
                    username: "admin".into(),
                    password: new_password.to_string(),
                },
            )
            .await;
        assert_status(&new_password, StatusCode::OK);
        assert!(
            !new_password.parse::<SignInResponse>().token.is_empty(),
            "{}",
            new_password.body
        );
    }

    async fn isolated_api(label: &str) -> ApiClient {
        let (database, provider) = matrix_cell();
        let label = format!("{database}-{provider}-{label}");
        let spawned = spawn_instance(&database, &provider, &label, &[]).await;
        ApiClient::new(spawned.base_url)
    }

    async fn bootstrap_admin(api: &ApiClient, password: &str) -> String {
        let res = api
            .post(
                "/v1/auth/sign-up",
                Auth::None,
                SignUpRequest {
                    password: password.to_string(),
                },
            )
            .await;
        assert_status(&res, StatusCode::OK);
        res.parse::<SignInResponse>().token
    }
}

mod protected_routes {
    use super::*;

    #[tokio::test]
    async fn reject_missing_credentials() {
        let app = app().await;
        let res = app.api().get("/v1/me", Auth::None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn reject_a_malformed_bearer_token() {
        let app = app().await;
        let res = app.api().get("/v1/me", Auth::Bearer("not-a-jwt")).await;
        assert_error(&res, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn accept_a_valid_token() {
        let app = app().await;
        let token = app.admin_token().await;
        let res = app.api().get("/v1/me", Auth::Bearer(token)).await;
        assert_status(&res, StatusCode::OK);
    }
}

mod local_accounts {
    use super::*;
    use crate::common::fixtures::unique;
    use serde_json::json;
    use swissknife_types::{Account, CreateAccountRequest, LocalLoginReset, Permission};

    const PASSWORD: &str = "local-account-test-passphrase";

    async fn account(username: &str) -> (Account, String) {
        let app = app().await;
        let admin = app.admin_token().await;
        let created = app
            .api()
            .post(
                "/v1/accounts",
                Auth::Bearer(admin),
                CreateAccountRequest {
                    display_name: Some("Local account".into()),
                    permissions: vec![],
                },
            )
            .await;
        assert_status(&created, StatusCode::OK);
        let account = created.parse::<Account>();
        let login = app
            .api()
            .post(
                &format!("/v1/accounts/{}/local-login", account.id),
                Auth::Bearer(admin),
                json!({"username": username}),
            )
            .await;
        assert_status(&login, StatusCode::OK);
        let grant = login.parse::<LocalLoginReset>();
        let reset = app
            .api()
            .post(
                "/v1/auth/reset-password",
                Auth::None,
                json!({"code": grant.code, "new_password": PASSWORD}),
            )
            .await;
        assert_status(&reset, StatusCode::NO_CONTENT);
        (account, sign_in(username, PASSWORD).await)
    }

    async fn sign_in(username: &str, password: &str) -> String {
        let response = app()
            .await
            .api()
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                json!({"username": username, "password": password}),
            )
            .await;
        assert_status(&response, StatusCode::OK);
        response.parse::<SignInResponse>().token
    }

    #[tokio::test]
    async fn concurrent_username_claims_have_one_winner() {
        let app = app().await;
        let admin = app.admin_token().await;
        let first = app.create_account(admin, "First login candidate").await;
        let second = app.create_account(admin, "Second login candidate").await;
        let username = unique("duplicate");
        let api = app.api();
        let responses = join_all([first.id, second.id].into_iter().map(|id| {
            let api = api.clone();
            let username = username.clone();
            async move {
                api.post(
                    &format!("/v1/accounts/{id}/local-login"),
                    Auth::Bearer(admin),
                    json!({"username":username}),
                )
                .await
            }
        }))
        .await;
        assert_eq!(responses.iter().filter(|r| r.status == StatusCode::OK).count(), 1);
        assert_eq!(responses.iter().filter(|r| r.status == StatusCode::CONFLICT).count(), 1);
    }

    #[tokio::test]
    async fn replacing_or_disabling_a_reset_invalidates_earlier_codes() {
        let username = unique("reset-grants");
        let (account, _) = account(&username).await;
        let app = app().await;
        let admin = app.admin_token().await;
        let reset_path = format!("/v1/accounts/{}/local-login/reset", account.id);
        let first = app
            .api()
            .post(&reset_path, Auth::Bearer(admin), json!({}))
            .await
            .parse::<LocalLoginReset>();
        let second = app
            .api()
            .post(&reset_path, Auth::Bearer(admin), json!({}))
            .await
            .parse::<LocalLoginReset>();
        assert_error(
            &app.api()
                .post(
                    "/v1/auth/reset-password",
                    Auth::None,
                    json!({"code":first.code,"new_password":PASSWORD}),
                )
                .await,
            StatusCode::UNAUTHORIZED,
        );
        let disabled = app
            .api()
            .put(
                &format!("/v1/accounts/{}/local-login", account.id),
                Auth::Bearer(admin),
                json!({"enabled":false}),
            )
            .await;
        assert_status(&disabled, StatusCode::NO_CONTENT);
        assert_error(
            &app.api()
                .post(
                    "/v1/auth/reset-password",
                    Auth::None,
                    json!({"code":second.code,"new_password":PASSWORD}),
                )
                .await,
            StatusCode::UNAUTHORIZED,
        );
    }

    #[tokio::test]
    async fn independent_credentials_keep_the_account_and_wallet_ownership() {
        let username = unique("local");
        let (account, token) = account(&username).await;
        let app = app().await;
        let me = app.api().get("/v1/me", Auth::Bearer(&token)).await;
        assert_status(&me, StatusCode::OK);
        let me = me.parse::<Account>();
        assert_eq!(me.id, account.id);
        assert_eq!(me.identity.unwrap().subject, username);
        assert_eq!(me.wallets.len(), 1);
        assert_eq!(me.wallets[0].account_id, account.id);
        let admin_profile = app
            .api()
            .get("/v1/me", Auth::Bearer(app.admin_token().await))
            .await
            .parse::<Account>();
        assert_ne!(me.id, admin_profile.id);
        let forbidden = app.api().get("/v1/accounts", Auth::Bearer(&token)).await;
        assert_error(&forbidden, StatusCode::FORBIDDEN);
        let profile = app
            .api()
            .put(
                &format!("/v1/accounts/{}", account.id),
                Auth::Bearer(app.admin_token().await),
                json!({"display_name":"Renamed account"}),
            )
            .await;
        assert_status(&profile, StatusCode::OK);
        let renamed_token = sign_in(&username.to_ascii_uppercase(), PASSWORD).await;
        let renamed = app
            .api()
            .get("/v1/me", Auth::Bearer(&renamed_token))
            .await
            .parse::<Account>();
        assert_eq!(renamed.id, account.id);
        assert_eq!(renamed.display_name.as_deref(), Some("Renamed account"));
        assert_eq!(renamed.wallets[0].id, me.wallets[0].id);
    }

    #[tokio::test]
    async fn password_change_is_scoped_to_the_caller_and_revokes_old_sessions() {
        let username = unique("change");
        let (_, token) = account(&username).await;
        let app = app().await;
        let changed = app
            .api()
            .post(
                "/v1/auth/change-password",
                Auth::Bearer(&token),
                json!({"current_password":PASSWORD,"new_password":"changed-local-passphrase"}),
            )
            .await;
        assert_status(&changed, StatusCode::NO_CONTENT);
        let old_session = app.api().get("/v1/me", Auth::Bearer(&token)).await;
        assert_error(&old_session, StatusCode::UNAUTHORIZED);
        let old_password = app
            .api()
            .post(
                "/v1/auth/sign-in",
                Auth::None,
                json!({"username":username,"password":PASSWORD}),
            )
            .await;
        assert_error(&old_password, StatusCode::UNAUTHORIZED);
        sign_in(&username, "changed-local-passphrase").await;
        sign_in("admin", ADMIN_PASSWORD).await;
    }

    #[tokio::test]
    async fn disabling_local_login_revokes_sessions_but_keeps_api_keys_and_wallets() {
        let username = unique("disable");
        let (account, token) = account(&username).await;
        let app = app().await;
        let admin = app.admin_token().await;
        let key = app.account_api_key(admin, account.id, vec![]).await;
        let disabled = app
            .api()
            .put(
                &format!("/v1/accounts/{}/local-login", account.id),
                Auth::Bearer(admin),
                json!({"enabled":false}),
            )
            .await;
        assert_status(&disabled, StatusCode::NO_CONTENT);
        assert_error(
            &app.api().get("/v1/me", Auth::Bearer(&token)).await,
            StatusCode::UNAUTHORIZED,
        );
        assert_error(
            &app.api()
                .post(
                    "/v1/auth/sign-in",
                    Auth::None,
                    json!({"username":username,"password":PASSWORD}),
                )
                .await,
            StatusCode::UNAUTHORIZED,
        );
        let keyed = app.api().get("/v1/me", Auth::ApiKey(&key)).await;
        assert_status(&keyed, StatusCode::OK);
        assert_eq!(keyed.parse::<Account>().id, account.id);
        let enabled = app
            .api()
            .put(
                &format!("/v1/accounts/{}/local-login", account.id),
                Auth::Bearer(admin),
                json!({"enabled":true}),
            )
            .await;
        assert_status(&enabled, StatusCode::NO_CONTENT);
        assert_error(
            &app.api().get("/v1/me", Auth::Bearer(&token)).await,
            StatusCode::UNAUTHORIZED,
        );
        sign_in(&username, PASSWORD).await;
    }

    #[tokio::test]
    async fn reset_code_is_single_use_and_replaces_the_password() {
        let username = unique("reset");
        let (account, token) = account(&username).await;
        let app = app().await;
        let grant = app
            .api()
            .post(
                &format!("/v1/accounts/{}/local-login/reset", account.id),
                Auth::Bearer(app.admin_token().await),
                json!({}),
            )
            .await;
        assert_status(&grant, StatusCode::OK);
        let grant = grant.parse::<LocalLoginReset>();
        assert_error(
            &app.api().get("/v1/me", Auth::Bearer(&token)).await,
            StatusCode::UNAUTHORIZED,
        );
        assert_error(
            &app.api()
                .post(
                    "/v1/auth/sign-in",
                    Auth::None,
                    json!({"username":username,"password":PASSWORD}),
                )
                .await,
            StatusCode::UNAUTHORIZED,
        );
        let api = app.api();
        let responses = join_all((0..2).map(|_| {
            api.post(
                "/v1/auth/reset-password",
                Auth::None,
                json!({"code":grant.code, "new_password":"reset-local-passphrase"}),
            )
        }))
        .await;
        assert_eq!(
            responses.iter().filter(|r| r.status == StatusCode::NO_CONTENT).count(),
            1
        );
        assert!(responses.iter().all(|r| matches!(
            r.status,
            StatusCode::NO_CONTENT | StatusCode::CONFLICT | StatusCode::UNAUTHORIZED
        )));
        sign_in(&username, "reset-local-passphrase").await;
        let replay = app
            .api()
            .post(
                "/v1/auth/reset-password",
                Auth::None,
                json!({"code":grant.code, "new_password":PASSWORD}),
            )
            .await;
        assert_error(&replay, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn deleted_local_tokens_cannot_recreate_accounts_or_follow_reused_usernames() {
        let username = unique("delete");
        let (first, token) = account(&username).await;
        let app = app().await;
        let deleted = app
            .api()
            .delete(
                &format!("/v1/accounts/{}", first.id),
                Auth::Bearer(app.admin_token().await),
            )
            .await;
        assert_status(&deleted, StatusCode::OK);
        assert_error(
            &app.api().get("/v1/me", Auth::Bearer(&token)).await,
            StatusCode::UNAUTHORIZED,
        );
        let (second, _) = account(&username).await;
        assert_ne!(first.id, second.id);
        assert_error(
            &app.api().get("/v1/me", Auth::Bearer(&token)).await,
            StatusCode::UNAUTHORIZED,
        );
        assert_error(
            &app.api()
                .get(
                    &format!("/v1/accounts/{}", first.id),
                    Auth::Bearer(app.admin_token().await),
                )
                .await,
            StatusCode::NOT_FOUND,
        );
    }

    #[tokio::test]
    async fn credential_management_requires_account_administration_and_uses_current_permissions() {
        let username = unique("permissions");
        let (account, token) = account(&username).await;
        let app = app().await;
        let path = format!("/v1/accounts/{}/local-login/reset", account.id);
        assert_error(
            &app.api().post(&path, Auth::Bearer(&token), json!({})).await,
            StatusCode::FORBIDDEN,
        );
        assert_error(
            &app.api()
                .get(
                    &format!("/v1/accounts/{}/local-login", account.id),
                    Auth::Bearer(&token),
                )
                .await,
            StatusCode::FORBIDDEN,
        );
        let granted = app
            .api()
            .put(
                &format!("/v1/accounts/{}/permissions", account.id),
                Auth::Bearer(app.admin_token().await),
                json!({"permissions":[Permission::ReadAccount]}),
            )
            .await;
        assert_status(&granted, StatusCode::OK);
        assert_status(
            &app.api().get("/v1/accounts", Auth::Bearer(&token)).await,
            StatusCode::OK,
        );
        let revoked = app
            .api()
            .put(
                &format!("/v1/accounts/{}/permissions", account.id),
                Auth::Bearer(app.admin_token().await),
                json!({"permissions":[]}),
            )
            .await;
        assert_status(&revoked, StatusCode::OK);
        assert_error(
            &app.api().get("/v1/accounts", Auth::Bearer(&token)).await,
            StatusCode::FORBIDDEN,
        );
    }
}

#[tokio::test]
async fn operator_recovery_targets_an_existing_account_without_reopening_setup() {
    use serde_json::json;
    use swissknife_types::Account;
    let (database, provider) = matrix_cell();
    let spawned = spawn_instance(
        &database,
        &provider,
        &format!("{database}-{provider}-local-recovery"),
        &[],
    )
    .await;
    let api = ApiClient::new(spawned.base_url);
    let sign_up = api
        .post("/v1/auth/sign-up", Auth::None, json!({"password":ADMIN_PASSWORD}))
        .await;
    assert_status(&sign_up, StatusCode::OK);
    let token = sign_up.parse::<SignInResponse>().token;
    let before = api.get("/v1/me", Auth::Bearer(&token)).await.parse::<Account>();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_swissknife"))
        .args(["recover-local-login", &before.id.to_string()])
        .env("RUN_MODE", "itest")
        .env("SWISSKNIFE_DATABASE__URL", spawned.database_url)
        .output()
        .expect("run operator recovery");
    assert!(
        output.status.success(),
        "recovery command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let code = stdout.lines().last().unwrap();
    assert_eq!(code.len(), 43);
    assert_error(&api.get("/v1/me", Auth::Bearer(&token)).await, StatusCode::UNAUTHORIZED);
    let reset = api
        .post(
            "/v1/auth/reset-password",
            Auth::None,
            json!({"code":code,"new_password":"recovered-admin-password"}),
        )
        .await;
    assert_status(&reset, StatusCode::NO_CONTENT);
    let login = api
        .post(
            "/v1/auth/sign-in",
            Auth::None,
            json!({"username":"admin","password":"recovered-admin-password"}),
        )
        .await;
    assert_status(&login, StatusCode::OK);
    let after = api
        .get("/v1/me", Auth::Bearer(&login.parse::<SignInResponse>().token))
        .await
        .parse::<Account>();
    assert_eq!(before.id, after.id);
    assert_eq!(before.wallets[0].id, after.wallets[0].id);
    assert_eq!(before.permissions, after.permissions);
    assert_error(
        &api.post("/v1/auth/sign-up", Auth::None, json!({"password":ADMIN_PASSWORD}))
            .await,
        StatusCode::CONFLICT,
    );
}

#[tokio::test]
async fn local_tokens_without_credential_binding_require_sign_in_after_upgrade() {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    let app = app().await;
    app.admin_token().await;
    let legacy = encode(&Header::default(), &json!({
        "sub":"admin", "iat":chrono::Utc::now().timestamp(), "exp":chrono::Utc::now().timestamp() + 3600, "permissions":[]
    }), &EncodingKey::from_secret(b"integration-test-secret")).unwrap();
    assert_error(
        &app.api().get("/v1/me", Auth::Bearer(&legacy)).await,
        StatusCode::UNAUTHORIZED,
    );
}
