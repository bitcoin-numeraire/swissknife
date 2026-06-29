# Local Mock OAuth2

SwissKnife can use this local OpenID provider for development without Auth0 or
another external OAuth2 service.

Start it with:

```bash
make up-oauth2
```

Point the backend at the default issuer and audience:

```bash
SWISSKNIFE_AUTH_PROVIDER=oauth2
SWISSKNIFE_OAUTH2__DOMAIN=http://127.0.0.1:8090/default
SWISSKNIFE_OAUTH2__AUDIENCE=https://swissknife.local/api
```

Fetch a token for a persona:

```bash
make oauth2-token OAUTH2_PERSONA=dev-admin
```

Use the dashboard persona picker with:

```bash
NEXT_PUBLIC_AUTH_METHOD=mock-oauth2
NEXT_PUBLIC_MOCK_OAUTH2_TOKEN_URL=http://127.0.0.1:8090/default/token
NEXT_PUBLIC_MOCK_OAUTH2_CLIENT_SECRET=dev-secret
```

Available personas:

- `dev-admin`: all permissions
- `dev-readonly`: all read permissions
- `dev-wallet-operator`: wallet and transaction read/write permissions
- `dev-address-operator`: lightning and bitcoin address permissions
- `dev-api-key-admin`: API key read/write permissions
- `dev-empty`: authenticated user with no permissions
