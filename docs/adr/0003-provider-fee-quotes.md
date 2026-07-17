# ADR 0003: Provider-derived payment fee quotes and hard fee limits

- Status: Accepted
- Date: 2026-07-17
- Issues: #248, #294, #300

## Context

SwissKnife used a flat `fee_buffer` when reserving outgoing Lightning payments. The buffer was neither a route quote nor a payment limit: it could reserve too much on cheap routes, too little on expensive routes, and it did not stop a node from paying a larger fee.

A useful pre-payment experience needs to distinguish two values:

- the **estimated fee**, based on the route graph at quote time;
- the **maximum fee**, an absolute cap enforced by the Lightning provider.

Lightning graph estimates cannot be guaranteed. Channel balances are private, routes can change between quote and execution, and a provider can retry over another route. On-chain transaction preparation is more precise, but a separately requested quote can still become stale before the final transaction is prepared.

## Decision

### Lightning

`LnClient` exposes a provider-specific graph estimate and an amount-aware hard fee policy. Payment execution receives that hard limit explicitly.

- LND uses `EstimateRouteFee` with destination plus amount, which selects its fast graph-based mode.
- Core Lightning uses `getroutes` with `auto.localchans` and `auto.sourcefree`, one part, and the configured fee budget.
- LND keeps its configured absolute `fee_limit_msat`.
- Core Lightning keeps its configured `maxfee`; when omitted, SwissKnife mirrors `xpay`'s `max(1%, 5000 msat)` default.

The payment service reserves `amount + maximum_fee`, not merely `amount + estimated_fee`. This prevents two simultaneous payments from spending the same fee headroom when execution legitimately chooses a route above the estimate. After settlement, the existing unit of work reconciles the reservation to the node's actual fee and releases the remainder.

If route estimation fails, fee quoting and payment admission fall back to the provider's hard maximum. The API returns no expected fee in that case, while still returning the guaranteed cap. An unavailable graph route does not prevent the provider from attempting private hints, retries, or a route that appears later.

If a provider estimate is already above the hard maximum, SwissKnife rejects the quote and payment before reserving funds. Such an estimate cannot succeed under the configured execution policy.

### On-chain and internal payments

For on-chain payments, the quote endpoint prepares a transaction with the configured wallet, reports its fee, then immediately releases the temporary input lease. The send path prepares the transaction again and reserves its then-current exact fee. The quote is therefore presented as an estimate, not a guarantee across time.

Internal transfers return zero for both values.

### API and dashboard

Both administrative and wallet-scoped APIs expose the same quote shape:

- ledger and payment amount;
- optional estimated fee and total;
- maximum fee and total.

The send drawer requests a quote only when the user asks for it. It shows the expected fee when available and an "up to" value for Lightning when the hard limit is higher. This keeps the ordinary Lightning flow lightweight while making the more material on-chain fee visible before confirmation.

The quote is informational. The execution-time maximum is guaranteed for Lightning because the same policy is passed to the provider; the estimated route fee is not guaranteed. On-chain execution uses the fee of the final prepared transaction, which may differ from an earlier quote.

## Consequences

- The hidden `fee_buffer` configuration is removed.
- Lightning admission reserves slightly more than the expected fee, but only up to an explicit and typically small provider cap.
- Route-estimation failure degrades to a clearly represented maximum instead of blocking payments.
- Quote requests for LNURL/Lightning Address inputs perform the LNURL callback needed to obtain a BOLT11 invoice.
- Provider estimates remain graph snapshots. A future durable quote token could avoid re-estimation, but it is unnecessary for the current user-triggered flow.
