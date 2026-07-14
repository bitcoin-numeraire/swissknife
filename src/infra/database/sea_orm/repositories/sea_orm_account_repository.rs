use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, LoaderTrait, ModelTrait,
    QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set, TransactionTrait,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    application::errors::DatabaseError,
    domains::{
        account::{Account, AccountFilter, AccountPreferences, AccountRepository, AuthProvider, Permission},
        payment::PaymentStatus,
        wallet::Wallet as AccountWallet,
    },
    infra::database::sea_orm::models::{
        account, account_preference, auth_identity,
        invoice::Column as InvoiceColumn,
        payment::Column as PaymentColumn,
        prelude::{
            Account as AccountEntity, AccountPreference, Asset as AssetEntity, AuthIdentity, Invoice as InvoiceEntity,
            LnAddress as LnAddressEntity, Payment as PaymentEntity, Wallet as WalletEntity,
        },
        wallet,
    },
    infra::database::sea_orm::sea_order,
};

#[derive(Clone)]
pub struct SeaOrmAccountRepository {
    db: DatabaseConnection,
}

impl SeaOrmAccountRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    async fn hydrate_wallets(
        &self,
        wallet_groups: Vec<Vec<wallet::Model>>,
    ) -> Result<Vec<Vec<AccountWallet>>, DatabaseError> {
        let group_lengths = wallet_groups.iter().map(Vec::len).collect::<Vec<_>>();
        let models = wallet_groups.into_iter().flatten().collect::<Vec<_>>();
        if models.is_empty() {
            return Ok(group_lengths.into_iter().map(|_| Vec::new()).collect());
        }

        let assets = models
            .load_one(AssetEntity, &self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let ln_addresses = models
            .load_one(LnAddressEntity, &self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;

        let wallet_ids = models.iter().map(|model| model.id).collect::<Vec<_>>();
        let mut received_by_wallet = InvoiceEntity::find()
            .filter(InvoiceColumn::WalletId.is_in(wallet_ids.clone()))
            .select_only()
            .column(InvoiceColumn::WalletId)
            .column_as(
                Expr::cust("CAST(SUM(invoice.amount_received_msat) AS BIGINT)"),
                "received_msat",
            )
            .group_by(InvoiceColumn::WalletId)
            .into_tuple::<(Uuid, Option<i64>)>()
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?
            .into_iter()
            .collect::<HashMap<_, _>>();
        let mut spent_by_wallet = PaymentEntity::find()
            .filter(PaymentColumn::WalletId.is_in(wallet_ids))
            .filter(PaymentColumn::Status.eq(PaymentStatus::Settled.to_string()))
            .select_only()
            .column(PaymentColumn::WalletId)
            .column_as(Expr::cust("CAST(SUM(payment.amount_msat) AS BIGINT)"), "sent_msat")
            .column_as(Expr::cust("CAST(SUM(payment.fee_msat) AS BIGINT)"), "fees_paid_msat")
            .group_by(PaymentColumn::WalletId)
            .into_tuple::<(Uuid, Option<i64>, Option<i64>)>()
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?
            .into_iter()
            .map(|(id, sent, fees)| (id, (sent, fees)))
            .collect::<HashMap<_, _>>();

        let mut wallets = Vec::with_capacity(models.len());
        for ((wallet_model, asset_model), ln_address_model) in models.into_iter().zip(assets).zip(ln_addresses) {
            let mut wallet: AccountWallet = wallet_model.into();
            let received_msat = received_by_wallet.remove(&wallet.id).flatten().unwrap_or(0);
            let (sent_msat, fees_paid_msat) = spent_by_wallet.remove(&wallet.id).unwrap_or((None, None));
            wallet.balance.received_msat = received_msat as u64;
            wallet.balance.sent_msat = sent_msat.unwrap_or(0) as u64;
            wallet.balance.fees_paid_msat = fees_paid_msat.unwrap_or(0) as u64;
            wallet.asset = asset_model.map(Into::into);
            wallet.ln_address = ln_address_model.map(Into::into);
            wallets.push(wallet);
        }

        let mut wallets = wallets.into_iter();
        Ok(group_lengths
            .into_iter()
            .map(|length| wallets.by_ref().take(length).collect())
            .collect())
    }
}

#[async_trait]
impl AccountRepository for SeaOrmAccountRepository {
    async fn find(&self, id: Uuid) -> Result<Option<Account>, DatabaseError> {
        let Some((account_model, preference_model)) = AccountEntity::find_by_id(id)
            .find_also_related(AccountPreference)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        else {
            return Ok(None);
        };

        let identity = account_model
            .find_related(AuthIdentity)
            .order_by_asc(auth_identity::Column::CreatedAt)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;
        let wallet_models = account_model
            .find_related(WalletEntity)
            .order_by_asc(wallet::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;

        let preference_model = preference_model
            .ok_or_else(|| DatabaseError::FindRelated("account does not reference preferences".to_string()))?;

        let mut account: Account = account_model.into();
        account.identity = identity.map(Into::into);
        account.preferences = Some(preference_model.into());
        account.wallets = self
            .hydrate_wallets(vec![wallet_models])
            .await?
            .pop()
            .unwrap_or_default();

        Ok(Some(account))
    }

    async fn find_by_identity(&self, provider: AuthProvider, subject: &str) -> Result<Option<Account>, DatabaseError> {
        let identity_with_account = AuthIdentity::find()
            .filter(auth_identity::Column::Provider.eq(provider.to_string()))
            .filter(auth_identity::Column::Subject.eq(subject))
            .find_also_related(AccountEntity)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?;

        let (identity, account_model) = match identity_with_account {
            Some((identity, Some(account))) => (identity, account),
            Some((_, None)) => {
                return Err(DatabaseError::FindRelated(
                    "auth identity does not reference an account".to_string(),
                ));
            }
            None => return Ok(None),
        };

        let preference_model = account_model
            .find_related(AccountPreference)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let wallet_models = account_model
            .find_related(WalletEntity)
            .order_by_asc(wallet::Column::CreatedAt)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;

        let mut account: Account = account_model.into();
        account.identity = Some(identity.into());
        account.preferences = preference_model.map(Into::into);
        account.wallets = self
            .hydrate_wallets(vec![wallet_models])
            .await?
            .pop()
            .unwrap_or_default();

        Ok(Some(account))
    }

    async fn find_many(&self, filter: AccountFilter) -> Result<Vec<Account>, DatabaseError> {
        let models = AccountEntity::find()
            .apply_if(filter.ids, |query, ids| query.filter(account::Column::Id.is_in(ids)))
            .order_by(account::Column::CreatedAt, sea_order(&filter.order_direction))
            .offset(filter.offset)
            .limit(filter.limit)
            .all(&self.db)
            .await
            .map_err(|e| DatabaseError::FindMany(e.to_string()))?;

        if models.is_empty() {
            return Ok(Vec::new());
        }

        let identities = models
            .load_many(
                AuthIdentity::find().order_by_asc(auth_identity::Column::CreatedAt),
                &self.db,
            )
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let preferences = models
            .load_one(AccountPreference, &self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let wallet_models = models
            .load_many(WalletEntity::find().order_by_asc(wallet::Column::CreatedAt), &self.db)
            .await
            .map_err(|e| DatabaseError::FindRelated(e.to_string()))?;
        let wallets = self.hydrate_wallets(wallet_models).await?;

        let mut accounts = Vec::with_capacity(models.len());
        for (((model, identities), preference), wallets) in
            models.into_iter().zip(identities).zip(preferences).zip(wallets)
        {
            let preference = preference
                .ok_or_else(|| DatabaseError::FindRelated("account does not reference preferences".to_string()))?;
            let mut account: Account = model.into();
            account.identity = identities.into_iter().next().map(Into::into);
            account.preferences = Some(preference.into());
            account.wallets = wallets;
            accounts.push(account);
        }

        Ok(accounts)
    }

    async fn insert(
        &self,
        display_name: Option<String>,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let account_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        let mut permissions = Vec::new();
        for permission in initial_permissions {
            if !permissions.contains(permission) {
                permissions.push(permission.clone());
            }
        }
        let permissions = serde_json::to_value(permissions).map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let account_model = account::ActiveModel {
            id: Set(account_id),
            display_name: Set(display_name),
            permissions: Set(permissions),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let preference_model = account_preference::ActiveModel {
            account_id: Set(account_id),
            dashboard_settings: Set(json!({})),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let mut account: Account = account_model.into();
        account.preferences = Some(preference_model.into());
        Ok(account)
    }

    async fn upsert(
        &self,
        provider: AuthProvider,
        subject: &str,
        display_name: Option<String>,
        initial_permissions: &[Permission],
    ) -> Result<Account, DatabaseError> {
        let tx = self
            .db
            .begin()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let account_id = Uuid::new_v4();
        let identity_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();
        let mut permissions = Vec::new();
        for permission in initial_permissions {
            if !permissions.contains(permission) {
                permissions.push(permission.clone());
            }
        }
        let permissions_json = serde_json::to_value(permissions).map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let account_model = account::ActiveModel {
            id: Set(account_id),
            display_name: Set(display_name),
            permissions: Set(permissions_json),
            created_at: Set(now),
            ..Default::default()
        };

        let account_model = account_model
            .insert(&tx)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let preference_model = account_preference::ActiveModel {
            account_id: Set(account_id),
            dashboard_settings: Set(json!({})),
            created_at: Set(now),
            ..Default::default()
        };

        let preference_model = preference_model
            .insert(&tx)
            .await
            .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        let identity_model = auth_identity::ActiveModel {
            id: Set(identity_id),
            account_id: Set(account_id),
            provider: Set(provider.to_string()),
            subject: Set(subject.to_string()),
            created_at: Set(now),
        };

        let identity_insert = identity_model.insert(&tx).await;

        let identity_model = match identity_insert {
            Ok(identity_model) => identity_model,
            Err(err) => {
                // A concurrent first request can create this identity after the
                // caller's read. The unique index rejects our duplicate insert;
                // return the account that won the race.
                tx.rollback()
                    .await
                    .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

                return self
                    .find_by_identity(provider, subject)
                    .await?
                    .ok_or_else(|| DatabaseError::Insert(err.to_string()));
            }
        };

        tx.commit()
            .await
            .map_err(|e| DatabaseError::Transaction(e.to_string()))?;

        let mut account: Account = account_model.into();
        account.identity = Some(identity_model.into());
        account.preferences = Some(preference_model.into());

        Ok(account)
    }

    async fn update(&self, account: Account) -> Result<Account, DatabaseError> {
        let permissions = account
            .permissions
            .as_ref()
            .ok_or_else(|| DatabaseError::Update("account permissions are missing".to_string()))?;
        let permissions = serde_json::to_value(permissions).map_err(|e| DatabaseError::Update(e.to_string()))?;
        let identity = account.identity;
        let preferences = account.preferences;
        let wallets = account.wallets;

        let account_model = account::ActiveModel {
            id: Set(account.id),
            display_name: Set(account.display_name),
            permissions: Set(permissions),
            updated_at: Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }
        .update(&self.db)
        .await
        .map_err(|e| DatabaseError::Update(e.to_string()))?;

        let mut account: Account = account_model.into();
        account.identity = identity;
        account.preferences = preferences;
        account.wallets = wallets;
        Ok(account)
    }

    async fn update_preferences(
        &self,
        id: Uuid,
        dashboard_settings: Value,
    ) -> Result<Option<AccountPreferences>, DatabaseError> {
        let now = Utc::now().naive_utc();

        let Some(existing) = AccountPreference::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| DatabaseError::FindOne(e.to_string()))?
        else {
            return Ok(None);
        };

        let mut preference: account_preference::ActiveModel = existing.into();
        preference.dashboard_settings = Set(dashboard_settings);
        preference.updated_at = Set(Some(now));

        let preference = preference
            .update(&self.db)
            .await
            .map_err(|e| DatabaseError::Update(e.to_string()))?;

        Ok(Some(preference.into()))
    }

    async fn delete_many(&self, filter: AccountFilter) -> Result<u64, DatabaseError> {
        let result = AccountEntity::delete_many()
            .apply_if(filter.ids, |query, ids| query.filter(account::Column::Id.is_in(ids)))
            .exec(&self.db)
            .await
            .map_err(|e| DatabaseError::Delete(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
