# Dashboard browser tests

`yarn test:e2e` runs the existing mock OAuth2/dashboard suite. Webhook acceptance
uses a separate configuration because it requires the real SwissKnife API and
regtest providers, rather than successful API response mocks.

## Developers and webhook acceptance

From the repository root, start the integration dependencies and backend:

```sh
make itest-up
mkdir -p target/itest
RUN_MODE=itest \
SWISSKNIFE_AUTH_PROVIDER=oauth2 \
SWISSKNIFE_OAUTH2__DOMAIN=http://127.0.0.1:8090/default \
SWISSKNIFE_OAUTH2__AUDIENCE=https://swissknife.itest/api \
SWISSKNIFE_LN_PROVIDER=lnd_grpc \
SWISSKNIFE_WEB__ADDR=127.0.0.1:21993 \
SWISSKNIFE_HOST=http://127.0.0.1:21993 \
SWISSKNIFE_DATABASE__URL='sqlite://target/itest/dashboard-webhooks.db?mode=rwc' \
cargo run
```

Then, from `dashboard/`:

```sh
yarn test:e2e:webhooks
```

The suite starts a dashboard on **localhost:8180** and authenticates through the
integration stack's real mock OAuth2 issuer. It exercises permissionless owners,
read-only/write-only/read-write administrators, API-key regressions, filters,
pagination, wallet selection, keyboard/mobile layout and failed/revoked sessions.
Only explicit error scenarios intercept requests to inject a 503 or 401. CRUD,
authorization, persistence, queueing and diagnostics run against the real API.

Use an isolated test database: these tests create and delete regtest resources.
`SWISSKNIFE_E2E_API`, `SWISSKNIFE_E2E_OAUTH2` and
`SWISSKNIFE_ITEST_COMPOSE_PROJECT` override the API, issuer and compose project.
For non-default node ports, also pass the matching backend configuration overrides
as described in `tests/itest/README.md`. `PLAYWRIGHT_REUSE_SERVER=true` reuses an
already-running dashboard configured with the same environment.

## Signed HTTPS and settlement acceptance

The real HTTPS test is skipped unless both receiver variables are set. Start the
included signature-verifying receiver and expose it through a public HTTPS tunnel
(for example, a temporary Cloudflare tunnel):

```sh
export SWISSKNIFE_E2E_RECEIVER_FILE=/tmp/swissknife-webhook-receiver.json
node e2e/support/webhook-receiver.mjs
# In another terminal:
cloudflared tunnel --url http://127.0.0.1:8555
```

Pass the tunnel's current URL and the same capture-file path:

```sh
SWISSKNIFE_E2E_RECEIVER_URL=https://YOUR-TUNNEL.trycloudflare.com \
SWISSKNIFE_E2E_RECEIVER_FILE=/tmp/swissknife-webhook-receiver.json \
yarn test:e2e:webhooks --grep 'real HTTPS'
```

The browser creates a subscription, saves its one-time secret in a test-only local
file with mode 0600, and sends a test event. The test then creates an invoice and
pays it from the integration stack's CLN node. The receiver checks HMAC-SHA256 over
`timestamp.raw_body`, timestamp freshness and constant-time equality before
acknowledging either event. Assertions verify both captures and the dashboard's
resulting delivery history. No private-destination bypass is enabled.

Stop the receiver/tunnel and remove the capture file and its `.secret` companion
afterwards. Browser traces may contain these disposable test secrets; keep the
ignored `test-results/` artifacts local.
