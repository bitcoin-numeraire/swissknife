use std::collections::HashMap;

use swissknife_types::TransactionWalletDisplay;
use uuid::Uuid;

use crate::{
    application::errors::ApplicationError,
    domains::account::{Permission, User},
};

use super::WalletUseCases;

pub(crate) async fn administrative_summaries<T, R>(
    wallets: &dyn WalletUseCases,
    user: &User,
    transactions: Vec<T>,
    wallet_id: impl Fn(&T) -> Uuid,
    summary: impl Fn(T, Option<TransactionWalletDisplay>) -> R,
) -> Result<Vec<R>, ApplicationError> {
    let can_read_accounts = user.has_permission(Permission::ReadAccount);
    let displays = if can_read_accounts || user.has_permission(Permission::ReadWallet) {
        let mut ids: Vec<_> = transactions.iter().map(&wallet_id).collect();
        ids.sort_unstable();
        ids.dedup();
        wallets.transaction_displays(&ids, can_read_accounts).await?
    } else {
        Vec::new()
    };
    let by_wallet: HashMap<_, _> = displays
        .into_iter()
        .map(|display| (display.wallet_id, display))
        .collect();
    Ok(transactions
        .into_iter()
        .map(|transaction| {
            let display = by_wallet.get(&wallet_id(&transaction)).cloned();
            summary(transaction, display)
        })
        .collect())
}
