use std::str::FromStr;

use bitcoin::{
    hashes::{sha256, Hash},
    Address, Network,
};
use chrono::Utc;
use swissknife_types::SubmitPaymentRequest;
use uuid::Uuid;

use super::{
    payment_input::BitcoinAddressData, ClaimedWithdrawal, Payment, PaymentInstructions, PaymentService, PaymentStatus,
    Publication, SubmissionKey, SubmittedPayment, WithdrawalProgress,
};
use crate::application::{
    composition::Ledger,
    errors::{ApplicationError, DataError},
};
use crate::domains::bitcoin::BtcNetwork;

const RETRY_SECONDS: i64 = 30;

impl PaymentService {
    pub(super) async fn submit_withdrawal(
        &self,
        key: &SubmissionKey,
        request: SubmitPaymentRequest,
        destination: BitcoinAddressData,
    ) -> Result<SubmittedPayment, ApplicationError> {
        let network = self.bitcoin_wallet.network();
        self.ensure_wallet_network(key.wallet_id, network).await?;
        let bitcoin_network = match network {
            BtcNetwork::Bitcoin => Network::Bitcoin,
            BtcNetwork::Testnet => Network::Testnet,
            BtcNetwork::Testnet4 => Network::Testnet4,
            BtcNetwork::Signet => Network::Signet,
            BtcNetwork::Regtest | BtcNetwork::Simnet => Network::Regtest,
        };
        let address = Address::from_str(&destination.address)
            .map_err(|e| DataError::Validation(e.to_string()))?
            .require_network(bitcoin_network)
            .map_err(|e| DataError::Validation(e.to_string()))?;
        let uri_amount = destination
            .amount_sat
            .map(|n| {
                n.checked_mul(1000)
                    .ok_or_else(|| DataError::Validation("Bitcoin amount overflows".into()))
            })
            .transpose()?;
        if matches!((uri_amount,request.amount_msat),(Some(uri),Some(requested)) if uri != requested) {
            return Err(DataError::Validation("Submitted amount differs from Bitcoin URI".into()).into());
        }
        let amount = Self::validate_amount(uri_amount.or(request.amount_msat))?;
        if !amount.is_multiple_of(1000) {
            return Err(DataError::Validation("Bitcoin withdrawals require whole satoshis".into()).into());
        }
        if let Some(recipient) = self.store.btc_address.find_by_address(&destination.address).await? {
            let (payment, invoice) = Self::registered_bitcoin_transfer(
                key.wallet_id,
                recipient.wallet_id,
                destination.address,
                amount,
                request.comment.or(destination.message),
            )?;
            return self.store.payment_uow.submit_internal(key, payment, invoice).await;
        }
        if amount / 1000 < address.script_pubkey().minimal_non_dust().to_sat() {
            return Err(DataError::Validation("Withdrawal output is below the dust threshold".into()).into());
        }
        let cap = request
            .maximum_fee_msat
            .filter(|n| n.is_multiple_of(1000))
            .ok_or_else(|| DataError::Validation("Approve an explicit maximum miner fee in whole satoshis".into()))?;
        let reserve = Self::reserve_amount_msat(amount, cap)?;
        let payment = Payment {
            wallet_id: key.wallet_id,
            amount_msat: amount,
            status: PaymentStatus::Pending,
            ledger: Ledger::Onchain,
            description: request.comment.or(destination.message),
            ..Default::default()
        };
        self.store
            .payment_uow
            .submit(
                key,
                payment,
                reserve,
                PaymentInstructions::Onchain {
                    backend: self.bitcoin_wallet.withdrawal_backend(),
                    address: address.to_string(),
                    maximum_fee_msat: cap,
                },
            )
            .await
    }

    async fn release_funding(&self, owner: Uuid) -> Result<(), ApplicationError> {
        self.bitcoin_wallet
            .clear_withdrawal_attempt(sha256::Hash::hash(owner.as_bytes()).to_byte_array())
            .await?;
        Ok(())
    }

    pub(super) async fn execute_withdrawal(&self) -> Result<bool, ApplicationError> {
        let Some(claim) = self
            .store
            .payment_uow
            .claim_withdrawal(&self.bitcoin_wallet.withdrawal_backend())
            .await?
        else {
            return Ok(false);
        };
        let (progress, delay) = self.next_withdrawal_step(&claim).await?;
        if let Some(progress) = progress {
            self.store
                .payment_uow
                .advance_withdrawal(&claim, progress, delay)
                .await?;
        }
        Ok(true)
    }

    async fn next_withdrawal_step(
        &self,
        claim: &ClaimedWithdrawal,
    ) -> Result<(Option<WithdrawalProgress>, i64), ApplicationError> {
        use WithdrawalProgress as Step;
        let expired = Utc::now() >= claim.details.expires_at;
        let (next, delay) = match &claim.progress {
            Step::Accepted if expired => (Step::Expired, 0),
            Step::Accepted => (
                Step::Funding {
                    owner: claim.token,
                    waiting: false,
                },
                0,
            ),
            Step::Funding { owner, .. } | Step::Planned { owner, .. } if expired => {
                // LND also expires named leases itself, even if the original node is offline.
                if let Err(error) = if claim.original_backend {
                    self.release_funding(*owner).await
                } else {
                    Ok(())
                } {
                    tracing::warn!(%error, "Named withdrawal lease cleanup will use its provider expiry");
                }
                (Step::Expired, 0)
            }
            Step::Funding { owner, .. } => {
                // Recover any inputs leased before a lost funding reply. CLN planning is non-reserving.
                let funded = async {
                    self.release_funding(*owner).await?;
                    Ok::<_, ApplicationError>(
                        self.bitcoin_wallet
                            .plan_withdrawal(
                                claim.details.address.clone(),
                                claim.payment.amount_msat / 1000,
                                sha256::Hash::hash(owner.as_bytes()).to_byte_array(),
                            )
                            .await?,
                    )
                }
                .await;
                match funded {
                    Ok(plan)
                        if plan
                            .fee_sat
                            .checked_mul(1000)
                            .is_none_or(|fee| fee > claim.details.maximum_fee_msat) =>
                    {
                        if let Err(error) = self.release_funding(*owner).await {
                            tracing::warn!(%error, "Named withdrawal lease cleanup will use its provider expiry");
                        }
                        (
                            Step::Failed {
                                reason: "Miner fee exceeds the approved maximum".into(),
                            },
                            0,
                        )
                    }
                    Ok(plan) => (
                        Step::Planned {
                            owner: *owner,
                            plan,
                            waiting: false,
                        },
                        0,
                    ),
                    Err(error) => {
                        tracing::warn!(payment_id = %claim.payment.id, %error, "Withdrawal waiting for provider funding");
                        (
                            Step::Funding {
                                owner: *owner,
                                waiting: true,
                            },
                            RETRY_SECONDS,
                        )
                    }
                }
            }
            Step::Planned { owner, plan, .. } => match self
                .bitcoin_wallet
                .sign_withdrawal(plan, sha256::Hash::hash(owner.as_bytes()).to_byte_array())
                .await
            {
                Ok(transaction) => (
                    Step::Signed {
                        transaction,
                        fee_msat: plan.fee_sat * 1000,
                        publication: Publication::Ready,
                    },
                    0,
                ),
                Err(error) => {
                    tracing::warn!(payment_id = %claim.payment.id, %error, "Withdrawal signing deferred; retrying funding before the approval expires");
                    (
                        Step::Funding {
                            owner: *owner,
                            waiting: true,
                        },
                        RETRY_SECONDS,
                    )
                }
            },
            Step::Signed {
                transaction,
                fee_msat,
                publication,
            } => {
                // A previous process may have published this exact transaction before losing its reply.
                // Even if history is temporarily unavailable, replaying the stored signatures is idempotent.
                let txid = transaction.txid();
                let observed = self.bitcoin_wallet.get_transaction(&txid).await;
                let publication = match observed {
                    Ok(Some(tx)) if tx.txid == txid && tx.block_height.unwrap_or_default() > 0 => {
                        self.events.onchain_withdrawal(tx.withdrawal_event()).await?;
                        return Ok((None, 0));
                    }
                    Ok(Some(tx)) if tx.txid == txid => Publication::Broadcast,
                    Err(error) if !matches!(publication, Publication::Ready) => {
                        tracing::warn!(%txid, %error, "Waiting for withdrawal history");
                        Publication::Unknown
                    }
                    _ => match self.bitcoin_wallet.broadcast_transaction(transaction).await {
                        Ok(()) => Publication::Broadcast,
                        Err(error) => {
                            tracing::warn!(%txid, %error, "Withdrawal broadcast outcome unknown");
                            Publication::Unknown
                        }
                    },
                };
                (
                    Step::Signed {
                        transaction: transaction.clone(),
                        fee_msat: *fee_msat,
                        publication,
                    },
                    RETRY_SECONDS,
                )
            }
            Step::Failed { .. } | Step::Expired | Step::Cancelled | Step::Confirmed => {
                return Err(DataError::Inconsistency("Terminal withdrawal was claimed".into()).into())
            }
        };
        Ok((Some(next), delay))
    }
}
