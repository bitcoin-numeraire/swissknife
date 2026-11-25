# SwissKnife wallet MCP

Run `swissknife mcp --server https://wallet.example.com --wallet WALLET_UUID` in a client that supports MCP over stdio. Set `SWISSKNIFE_MCP_WALLET_KEY` through the MCP host’s secret environment. For a locally running API, HTTP on a loopback address is also supported. The process requires a reachable SwissKnife API supporting wallet credentials. It can run on a different machine from the node and database.

Create the credential from the owner account using `POST /v1/me/wallets/{wallet_id}/agent-keys`. Choose its expiry and independent `read`, `receive` and `spend` capabilities. Copy the issued secret once. Revoke it with `DELETE /v1/me/wallets/{wallet_id}/agent-keys/{key_id}`. Each connection selects one wallet. Start a separate configuration with its own wallet and secret for another account.

| Capability | Tools |
| --- | --- |
| Local decoding | `decode_invoice` |
| Read | `get_balance`, `get_wallet_info`, `get_lightning_address`, `list_contacts`, `list_payments`, `get_payment`, `get_payment_submission` |
| Receive | `create_invoice` |
| Spend | `send_payment`, `send_to_address`, `lnurl_pay` |

The resources `wallet://balance` and `wallet://payments/recent` return current wallet data. For payments, obtain authorization for the exact principal and fee cap in millisatoshis. Supply both as decimal strings. Choose a unique submission UUID and persist it with the terms before sending. Reuse those same values after a lost response. Poll `get_payment_submission` for settlement. A local transfer or unsupported destination returns an error. Invoice creation creates a new request on each call.

Host-side prompts and descriptions help users review transactions. A credential with spend permission can submit against the wallet’s available balance directly. Operators should fund a dedicated wallet or configure a server-side spending policy once supported.
