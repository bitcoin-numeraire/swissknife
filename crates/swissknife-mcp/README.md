# SwissKnife wallet MCP

Run `swissknife mcp --server https://wallet.example.com --wallet WALLET_UUID` in a client that supports MCP over stdio. Set `SWISSKNIFE_MCP_WALLET_KEY` through the MCP host’s secret environment. For a locally running API, HTTP on a loopback address is also supported. The process requires a reachable SwissKnife API supporting wallet credentials. It can run on a different machine from the node and database.

Create the credential from the owner account using `POST /v1/me/wallets/{wallet_id}/agent-keys`. Choose its expiry and the capabilities the client needs. The server checks each request against the selected wallet. Copy the issued secret once. Revoke it with `DELETE /v1/me/wallets/{wallet_id}/agent-keys/{key_id}`. Each connection selects one wallet. Start a separate configuration with its own wallet and secret for another account.

| Capability | Tools |
| --- | --- |
| Local decoding | `decode_invoice`, `decode_l402_challenge` |
| Read | `get_balance`, `get_wallet_info`, `get_lightning_address`, `list_contacts`, `list_saved_contacts`, `get_invoice`, `list_payments`, `get_payment`, `get_payment_submission`, `create_l402_authorization` |
| Manage contacts | `add_contact` |
| Receive | `create_invoice` |
| Spend | `send_payment`, `send_to_address`, `lnurl_pay` |

The resources `wallet://balance` and `wallet://payments/recent` return current wallet data. For payments, obtain authorization for the exact principal and fee cap in millisatoshis. Supply both as decimal strings. Choose a unique submission UUID and persist it with the terms before sending. Reuse those same values after a lost response. Poll `get_payment_submission` for settlement. A local transfer or unsupported destination returns an error. Save the invoice UUID returned by `create_invoice`. Call `get_invoice` with that UUID to check the current status, exact requested and received amounts, and settlement time. Repeating `create_invoice` creates another invoice.

Host-side prompts and descriptions help users review transactions. A credential with spend permission can submit against the wallet’s available balance directly. Fund the selected wallet according to the intended use and apply the spending limits supported by your SwissKnife deployment.

For a remote MCP client, use `--http-listen` on loopback and set a separate `SWISSKNIFE_MCP_HTTP_TOKEN` for the client connection. The HTTP transport supports resource subscriptions. See the [Compose and local npm examples](../../examples/mcp/README.md) for stdio clients.
