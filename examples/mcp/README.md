# Connect a local MCP host to SwissKnife

Choose a SwissKnife image built from a version that includes the `mcp` subcommand. Use a registry digest for a released image or build this checkout with `docker build --target swissknife-server -t swissknife-mcp:local .` from the repository root. Export that reference as `SWISSKNIFE_IMAGE`.

Place your deployment configuration in `production.toml` and the corresponding Lightning provider certificates in `certs/`, or set `SWISSKNIFE_CONFIG` and `SWISSKNIFE_CERTS` to existing paths. The provider endpoint must be reachable from the wallet container. Retain your JWT signing secret across restarts. The example persists SQLite in a named volume and exposes the wallet API on host loopback, default port 3000.

From this directory run `docker compose up -d wallet`. As the wallet owner, issue a scoped credential with an expiry through `POST /v1/me/wallets/{wallet_id}/agent-keys`. Begin with the permissions your client needs. Set `SWISSKNIFE_WALLET_ID` and provide `SWISSKNIFE_MCP_WALLET_KEY` in the MCP host's private environment.

Configure the host to execute this command, with the absolute path to the example:

```sh
docker compose -f /path/to/examples/mcp/compose.yaml run --rm --no-deps -T mcp
```

Protocol messages travel on standard input and output. Each MCP process connects to the wallet over its shared loopback interface. Its filesystem has neither provider credentials nor the wallet database. Revoke the scoped key to end access.

## Local npx launcher

If SwissKnife is installed on the host, point `SWISSKNIFE_BIN` to its executable or put `swissknife` on PATH. Pack the repository's dependency-free launcher once with `npm pack ./packages/swissknife-mcp` from the repository root. Configure the MCP host to use that local package:

```sh
npx --offline --yes --package /path/to/bitcoin-numeraire-swissknife-mcp-0.3.5.tgz \
  swissknife-mcp --server https://wallet.example.com --wallet YOUR_WALLET_UUID
```

The launcher forwards arguments and standard streams to `swissknife mcp`. The Rust process reads the scoped wallet credential from its environment. Keep credentials out of command arguments and committed JSON configuration.
