# SwissKnife wallet MCP

Run `swissknife mcp --server https://wallet.example.com --wallet WALLET_UUID` in a client that supports MCP over stdio. Set `SWISSKNIFE_MCP_WALLET_KEY` through the MCP host’s secret environment. For a locally running API, HTTP on a loopback address is also supported. The process requires a reachable SwissKnife API supporting wallet credentials. It can run on a different machine from the node and database.

Create the credential from the owner account using `POST /v1/me/wallets/{wallet_id}/agent-keys`. Choose its expiry and the capabilities the client needs. The server checks each request against the selected wallet. Copy the issued secret once. Revoke it with `DELETE /v1/me/wallets/{wallet_id}/agent-keys/{key_id}`. Each connection selects one wallet. Start a separate configuration with its own wallet and secret for another account.

| Capability | Tools |
| --- | --- |
| Local decoding | `decode_invoice`, `decode_l402_challenge` |
| Read | `get_balance`, `get_wallet_info`, `get_lightning_address`, `list_contacts`, `list_saved_contacts`, `get_invoice`, `list_payments`, `get_payment`, `get_payment_submission`, `create_l402_authorization` |
| Configured L402 resource | `check_l402_token` |
| Manage contacts | `add_contact` |
| Receive | `create_invoice` |
| Configured withdrawal origin | `inspect_lnurl_withdraw` |
| Read and configured withdrawal origin | `lnurl_withdraw` |
| Spend | `send_payment`, `send_to_address`, `lnurl_pay` |

The resources `wallet://balance` and `wallet://payments/recent` return current wallet data. For payments, obtain authorization for the exact principal and fee cap in millisatoshis. Supply both as decimal strings. Choose a unique submission UUID and persist it with the terms before sending. Reuse those same values after a lost response. Poll `get_payment_submission` for settlement. A local transfer or unsupported destination returns an error. Save the invoice UUID returned by `create_invoice`. Call `get_invoice` with that UUID to check the current status, exact requested and received amounts, and settlement time. Repeating `create_invoice` creates another invoice.

Host-side prompts and descriptions help users review transactions. A credential with spend permission can submit against the wallet’s available balance directly. Fund the selected wallet according to the intended use and apply the spending limits supported by your SwissKnife deployment.

For a remote MCP client, use `--http-listen` on loopback and set a separate `SWISSKNIFE_MCP_HTTP_TOKEN` for the client connection. Balance and recent-payment subscriptions on both transports retry temporary wallet API failures with increasing delays up to 30 seconds. After recovery the client receives an update if the wallet data changed since the last successful read. Revoking the credential stops the feed. See the [Compose and local npm examples](../../examples/mcp/README.md) for stdio clients.

## Filter outgoing payments

The read-only `list_payments` tool accepts `status` (`Pending`, `Settled`, `Failed`) and `ledger` (`Lightning`, `Internal`, `Onchain`). For example, `{"status":"Failed","ledger":"Lightning","limit":10}` returns up to ten failed Lightning payments from the selected wallet, newest first. The API applies both filters before `offset` and `limit`. Omit the filters to see outgoing payments on every ledger.

## Receive from a withdrawal service

Set `--withdraw-origin https://issuer.example` on the bridge to enable the voucher tools. Repeat the flag if the issuer uses a separate callback origin. A local service can use HTTP on an explicit loopback IP and port. The wallet key goes only to the wallet API.

Use `inspect_lnurl_withdraw` with the encoded voucher to review the issuer and permitted millisatoshi range. Create an invoice for the agreed amount with `create_invoice`, save its UUID, then give `lnurl_withdraw` the same voucher, `invoice_id` and `amount_msat`. An `accepted` callback means the issuer acknowledged the invoice. Use `get_invoice` to confirm receipt. After `rejected` or `unknown`, reconcile that same invoice before attempting redemption again.

## Check a cached L402 pass

Configure each trusted SwissKnife merchant resource with `--l402-resource https://merchant.example/l402/report`. Call `check_l402_token` with that exact `resource_url` and the cached `authorization`. The bridge verifies the preimage locally and presents a status-only macaroon to the issuer. It makes one HEAD request and never follows redirects.

An active request pack includes its remaining count. A time pass returns `remaining_requests: null`. An exhausted pack has zero uses. An inactive pass has expired or its route terms changed. An invalid proof fails verification. An unconfigured resource or an issuer without this protocol returns `unknown`. The count can change if another client uses the pass.
