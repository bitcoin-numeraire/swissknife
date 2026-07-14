# Numeraire SwissKnife Dashboard

Next.js dashboard for SwissKnife accounts and their asset-scoped wallets. The
dashboard uses the generated OpenAPI client under `src/lib/swissknife`; do not
edit generated client files manually.

Authenticated pages bootstrap the account and its wallets through
`src/contexts/account`. The header wallet selector controls the explicit wallet
ID used by balance, payment, invoice, contact, and Bitcoin-address requests.

## Development

Node 24 and Yarn 4 are required.

```bash
yarn install
yarn dev
```

The development server listens on <http://localhost:8080>. Point it at a backend
with `NEXT_PUBLIC_SERVER_URL`.

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
