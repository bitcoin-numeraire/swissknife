# SwissKnife wallet plugin

This plugin connects an MCP host to SwissKnife on loopback port 8452. Start the wallet bridge on the same machine as the host. Configure the bridge with the selected wallet's delegated key in `SWISSKNIFE_MCP_WALLET_KEY` and a separate random 32-byte hexadecimal token in `SWISSKNIFE_MCP_HTTP_TOKEN`. Give the host that transport token through its private environment.

```sh
swissknife mcp --server https://wallet.example.com --wallet YOUR_WALLET_UUID --http-listen 127.0.0.1:8452
```

In Claude Code, add the SwissKnife checkout as a local plugin marketplace, then install the wallet plugin.

```sh
claude plugin marketplace add /absolute/path/to/swissknife
claude plugin install swissknife-wallet@numeraire
```

OpenClaw 2026.11.10 recognizes the Claude bundle. Its embedded importer sends the bearer placeholder literally. Add the connection to the operator-managed profile so OpenClaw resolves the token from its environment. The single quotes below preserve the placeholder in the configuration.

```sh
openclaw mcp set swissknife-wallet '{"transport":"streamable-http","url":"http://127.0.0.1:8452/mcp","headers":{"Authorization":"Bearer ${SWISSKNIFE_MCP_HTTP_TOKEN}"}}'
openclaw mcp probe swissknife-wallet
```

Use the host's tool inventory to find the wallet balance and information tools. Test with a read-only wallet credential first. A wallet key with additional capabilities can create invoices, edit contacts or send payments. Account budgets apply to delegated payments. Revoke the key from the wallet to end access.

For multiple accounts, run one bridge per wallet with its own scoped key, transport token and loopback port. Add each connection under a distinct name in the MCP host and allow only the intended agents to access it.
