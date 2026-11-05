# On-chain listener recovery

SwissKnife saves the on-chain cursor after processing a batch successfully. If
projection fails, the listener retries from the saved cursor. Leave the cursor
in place so recovery can replay deposits and withdrawals that were not recorded.

For deployments with centralized logs, alert when a SwissKnife deployment logs
`Lightning listener failed; reconnecting` or
`Lightning listener replay sync failed; retrying` three or more times in five
minutes. Include the most recent `err` field in the notification. A single
reconnect can be transient, and an idle wallet can have a stationary cursor.
The HTTP health check probes the database and Lightning provider; it can remain
healthy while the listener repeatedly fails to record an event.

If PostgreSQL reports an `INT4`/`INT8` mismatch for `output_index`, `block_height`,
or `btc_block_height`, upgrade to a SwissKnife build with the on-chain column
migration. It runs before the listener starts and preserves the stored cursor.
The PostgreSQL type change can lock and rewrite the affected tables, so allow
time for it during rollout. After startup, verify the affected deposit by
outpoint and check that it produced one settled invoice and one balance credit.
