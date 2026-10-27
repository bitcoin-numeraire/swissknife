# Numeraire SwissKnife Dashboard

Next.js dashboard for SwissKnife accounts and their asset-scoped wallets. The
dashboard uses the generated OpenAPI client under `src/lib/swissknife`; do not
edit generated client files manually.

Authenticated pages bootstrap the account and its wallets through
`src/contexts/account`. The header wallet selector controls the explicit wallet
ID used by balance, payment, invoice, contact, and Bitcoin-address requests.

## Development

Use Node 24 and the Yarn 4 version pinned in `package.json` (via Corepack).

```bash
corepack enable
yarn install --immutable
cp .env.example .env.development.local
yarn dev
```

The development server listens on <http://localhost:8080>. Point it at a backend
with `NEXT_PUBLIC_SERVER_URL`, for example `http://localhost:3000`.

## Production configuration

The checked-in `.env.production` is the Numeraire hosted Auth0 profile. Set its
public values for your own deployment before building a production image:

| Setting                          | Purpose                                                              |
| -------------------------------- | -------------------------------------------------------------------- |
| `NEXT_PUBLIC_SERVER_URL`         | Public API origin; empty for the bundled, same-origin backend        |
| `NEXT_PUBLIC_AUTH_METHOD`        | `jwt` for the local owner login, or `auth0` for the hosted OIDC flow |
| `NEXT_PUBLIC_APPNAME`            | Display name                                                         |
| `NEXT_PUBLIC_AUTH0_DOMAIN`       | Auth0 tenant/custom domain                                           |
| `NEXT_PUBLIC_AUTH0_CLIENT_ID`    | Public SPA client ID                                                 |
| `NEXT_PUBLIC_AUTH0_CALLBACK_URL` | Registered dashboard callback URL                                    |
| `NEXT_PUBLIC_AUTH0_AUDIENCE`     | Must match the backend's OAuth2 audience                             |

`NEXT_PUBLIC_*` values are embedded at build time and visible in browser code;
do not put credentials or private provider secrets there. Setting these variables
only on a running container does not reconfigure the browser bundle. The Docker
builds copy `.env.production`; local override files are excluded from the Docker
context.

There are two build modes:

```bash
yarn build                           # Standalone Node.js server output
BUILD_STATIC_EXPORT=true yarn build  # Static export in out/ for the backend
```

From the repository root, `make build-docker-dashboard` builds the standalone
image on port 8080; `make build-docker` bundles the static export with the backend
on port 3000. See the root [README](../README.md) and
[upgrade guide](../docs/UPGRADING.md) for deployment.

## Verification

```bash
yarn lint
yarn typecheck
yarn test
yarn build
yarn fm:check
```

When backend DTOs or routes change, regenerate the OpenAPI document and client
from the repository root with `make openapi`.

The dashboard consumes account settlement events at `/v1/me/events` and
revalidates its REST state. Its stream requires a proxy that permits streaming
responses; REST remains authoritative after reconnects and cursor expiry.
