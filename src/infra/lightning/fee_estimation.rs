use std::future::Future;

use lightning_invoice::RouteHint;

use crate::{application::errors::LightningError, domains::payment::LnPaymentTarget};

/// Price the public prefix plus a complete hinted suffix. Each hinted quote is
/// a single path: otherwise splitting the prefix would undercount the suffix's
/// per-HTLC base fees. A quote is an estimate, not a liquidity guarantee; a
/// payment needing MPP across different hints can still use the provider cap.
pub(super) async fn estimate_with_hints<F, Fut>(
    target: LnPaymentTarget,
    source: &[u8],
    maximum_fee_msat: u64,
    quote: F,
) -> Result<u64, LightningError>
where
    F: Fn(LnPaymentTarget, u64, bool) -> Fut,
    Fut: Future<Output = Result<u64, LightningError>>,
{
    let mut candidates = vec![(
        LnPaymentTarget {
            route_hints: vec![],
            ..target.clone()
        },
        0,
        false,
    )];
    for hint in &target.route_hints {
        if let Some((prefix, fee)) = hinted_prefix(&target, hint, source) {
            if fee <= maximum_fee_msat {
                candidates.push((prefix, fee, true));
            }
        }
    }
    let mut best = None;
    let mut last_error = LightningError::EstimateFee("No route within the fee cap".to_string());
    for (prefix, suffix_fee, hinted) in candidates {
        let remaining_fee = maximum_fee_msat - suffix_fee;
        let result = if prefix.destination == source {
            Ok(0)
        } else {
            quote(prefix, remaining_fee, hinted).await
        };
        match result {
            Ok(fee) if fee <= remaining_fee => {
                let total = fee + suffix_fee;
                best = Some(best.map_or(total, |previous: u64| previous.min(total)));
            }
            Ok(_) => {}
            Err(err) => last_error = err,
        }
    }
    best.ok_or(last_error)
}

fn hinted_prefix(target: &LnPaymentTarget, hint: &RouteHint, source: &[u8]) -> Option<(LnPaymentTarget, u64)> {
    // If the payer is itself a hinted hop, only the suffix from there is needed.
    let start = hint
        .0
        .iter()
        .position(|hop| hop.src_node_id.serialize() == source)
        .unwrap_or(0);
    let hops = hint.0.get(start..)?;
    let first = hops.first()?;
    let mut amount_msat = target.amount_msat;
    let mut final_cltv_delta = target.final_cltv_delta;
    for hop in hops.iter().rev() {
        if hop.htlc_minimum_msat.is_some_and(|min| amount_msat < min)
            || hop.htlc_maximum_msat.is_some_and(|max| amount_msat > max)
        {
            return None;
        }
        // The source does not charge itself forwarding fees or forwarding CLTV.
        if hop.src_node_id.serialize() != source {
            let fee = u64::from(hop.fees.base_msat).checked_add(
                u64::try_from(u128::from(amount_msat) * u128::from(hop.fees.proportional_millionths) / 1_000_000)
                    .ok()?,
            )?;
            amount_msat = amount_msat.checked_add(fee)?;
            final_cltv_delta = final_cltv_delta.checked_add(u32::from(hop.cltv_expiry_delta))?;
        }
    }
    Some((
        LnPaymentTarget {
            destination: first.src_node_id.serialize().to_vec(),
            amount_msat,
            final_cltv_delta,
            route_hints: vec![],
        },
        amount_msat.checked_sub(target.amount_msat)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
    use lightning_invoice::{RouteHintHop, RoutingFees};

    fn key(n: u8) -> PublicKey {
        PublicKey::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[n; 32]).unwrap())
    }
    fn hop(n: u8, base: u32, ppm: u32) -> RouteHintHop {
        RouteHintHop {
            src_node_id: key(n),
            short_channel_id: u64::from(n),
            fees: RoutingFees {
                base_msat: base,
                proportional_millionths: ppm,
            },
            cltv_expiry_delta: 10,
            htlc_minimum_msat: None,
            htlc_maximum_msat: None,
        }
    }
    fn target(hints: Vec<RouteHint>) -> LnPaymentTarget {
        LnPaymentTarget {
            destination: key(9).serialize().to_vec(),
            amount_msat: 1_000_000,
            final_cltv_delta: 40,
            route_hints: hints,
        }
    }

    mod hinted_prefix {
        use super::*;
        #[test]
        fn compounds_each_hops_fees_and_cltv_backwards() {
            let hint = RouteHint(vec![hop(2, 1_000, 10_000), hop(3, 2_000, 20_000)]);
            let (prefix, fee) = hinted_prefix(&target(vec![]), &hint, &key(1).serialize()).unwrap();
            // Last hop: 22,000; first: 1,000 + 1% of 1,022,000.
            assert_eq!(fee, 33_220);
            assert_eq!(prefix.amount_msat, 1_033_220);
            assert_eq!(prefix.final_cltv_delta, 60);
            assert_eq!(prefix.destination, key(2).serialize());
        }
        #[test]
        fn skips_hops_before_the_payer_and_its_own_fee() {
            let hint = RouteHint(vec![
                hop(2, 1_000, 10_000),
                hop(3, 2_000, 20_000),
                hop(4, 3_000, 30_000),
            ]);
            let (prefix, fee) = hinted_prefix(&target(vec![]), &hint, &key(3).serialize()).unwrap();
            assert_eq!(fee, 33_000);
            assert_eq!(prefix.final_cltv_delta, 50);
            assert_eq!(prefix.destination, key(3).serialize());
        }
        #[test]
        fn rejects_limits_and_overflow_instead_of_wrapping() {
            let mut h = hop(2, 0, 0);
            h.htlc_minimum_msat = Some(1_000_001);
            assert!(hinted_prefix(&target(vec![]), &RouteHint(vec![h.clone()]), &[]).is_none());
            h.htlc_minimum_msat = None;
            h.htlc_maximum_msat = Some(999_999);
            assert!(hinted_prefix(&target(vec![]), &RouteHint(vec![h]), &[]).is_none());
            let mut t = target(vec![]);
            t.amount_msat = u64::MAX;
            assert!(hinted_prefix(&t, &RouteHint(vec![hop(2, 1, 0)]), &[]).is_none());
            t.amount_msat = 1_000_000;
            t.final_cltv_delta = u32::MAX;
            assert!(hinted_prefix(&t, &RouteHint(vec![hop(2, 0, 0)]), &[]).is_none());
        }
    }

    mod estimate_with_hints {
        use super::*;
        #[tokio::test]
        async fn uses_an_alternate_hint_when_public_destination_is_unknown() {
            let t = target(vec![
                RouteHint(vec![hop(2, 1_000, 2_500)]),
                RouteHint(vec![hop(3, 100, 100)]),
            ]);
            let fee = estimate_with_hints(t, &key(1).serialize(), 10_000, |prefix, cap, single_path| async move {
                if prefix.destination == key(9).serialize() {
                    return Err(LightningError::EstimateFee("unknown node".into()));
                }
                assert!(single_path);
                if prefix.destination == key(2).serialize() {
                    assert_eq!(prefix.amount_msat, 1_003_500);
                    assert_eq!(cap, 6_500);
                    Ok(1_234)
                } else {
                    Ok(500)
                }
            })
            .await
            .unwrap();
            assert_eq!(fee, 700);
        }
        #[tokio::test]
        async fn rejects_quotes_above_the_remaining_cap() {
            let t = target(vec![RouteHint(vec![hop(2, 1_000, 0)])]);
            assert!(estimate_with_hints(t, &[], 2_000, |_, _, _| async { Ok(2_001) })
                .await
                .is_err());
        }
        #[tokio::test]
        async fn prices_a_route_starting_at_the_payer_without_a_public_lookup() {
            let t = target(vec![RouteHint(vec![hop(1, 1_000, 10_000)])]);
            let fee = estimate_with_hints(t, &key(1).serialize(), 100, |_, _, _| async {
                Err(LightningError::EstimateFee("unknown destination".into()))
            })
            .await
            .unwrap();
            assert_eq!(fee, 0);
        }
    }
}
