# ADR 0004: Local JWT credentials and account management

- Status: Accepted
- Date: 2026-09-04
- Issue: [#338](https://github.com/bitcoin-numeraire/swissknife/issues/338)
- Extends: [ADR 0002](0002-identity-assets-wallet-model.md)

## Context

Local JWT authentication needs independent credentials and user management on top
of the account aggregate introduced by ADR 0002. Production deployments use Auth0;
there are no existing local JWT users to migrate.

This decision defines local login on top of the existing account ownership model.
An account remains the owner of permissions, API keys, preferences, and asset-scoped
wallets. Credentials authenticate access to that account; they do not own funds.

## Decision

### Identity and credentials

Each account can have at most one local login in this product slice. The login
links to `auth_identity(provider = jwt)` and uses a unique username as its subject.
Account UUIDs and identity UUIDs remain stable and distinct from display names.
Display names are editable, optional, and non-unique; changing one never changes
login, permissions, API keys, Lightning Addresses, or wallet ownership. Local
usernames are immutable after creation. Changing a username or linking multiple
login providers is a separate feature.

An account without a local credential can continue to own wallets and API keys.
Creating such an account does not give it a password or make it a local user.
OAuth2 claims remain authoritative for OAuth2 permissions; local requests load
permissions from `account.permissions` on every authenticated request.

### Password storage and lifecycle

Use a dedicated local-credential record linked to the account and its JWT
identity. Store only a salted password hash, an enabled/disabled flag, a random
credential revision, and lifecycle timestamps. Reset grants store only the hash
of a cryptographically random, expiring, single-use secret. Stored password and
reset-token hashes stay internal and are excluded from account DTOs and logs.

Passwords use Argon2id. Hashing and verification run outside the async executor
with bounded concurrency. Validate
password length before hashing, allow password-manager paste and passphrases,
and never truncate passwords silently. The concrete hash parameters, input
bounds, and login throttling are part of the implementation contract below.

A password change verifies the current password against the caller's own
credential and replaces it using a compare-and-swap on the credential revision.
An administrative reset revokes existing sessions and replaces any outstanding
reset grant. The recipient chooses a password using that grant, then signs in.
Reset never changes the username, account ID, permissions, or wallet ownership.
Disabling local login revokes sessions and outstanding reset grants. Re-enabling
requires an explicit administrator action and cannot revive an old token.

### Authentication and revocation

New local JWTs bind the username, immutable identity ID, and credential revision.
Authentication verifies the signature and expiry, resolves the existing identity
and credential, checks enabled state and revision, and loads the account's current
permissions. A missing local identity is unauthorized and is never provisioned
from a token. Local tokens require this credential binding. OAuth2 first-login
provisioning uses the identity provider's claims.

Password changes, resets, and login state transitions rotate the revision.
This invalidates all of that local identity's existing sessions on subsequent
requests, including sessions on other devices. Already admitted operations may
finish; this is not a mechanism for cancelling a published payment.

API keys retain their independent account-scoped grants. Password changes and
**Disable local login** do not revoke API keys. The administrator sees that
consequence and can separately revoke keys when removing a person's access.
Deleting an account removes its credentials and API keys through the account
aggregate's existing deletion path. Recreating the same username gets a new
identity ID and cannot accept tokens issued to the deleted account.

### API and administration

The dashboard keeps **Account Directory → Account Details** as the management
entry point. Create an account with its display name and explicit permissions,
then use **Create local login** on its detail page. New accounts default to no
administrative permissions. Multiple accounts may have `write:account`; there is
no database constraint or runtime rule that designates one permanent admin.

| Operation | Contract |
| --- | --- |
| `POST /v1/auth/sign-up` | One-time owner setup; creates `jwt/admin`, its account and credentials, and a permanent setup marker in one transaction. |
| `POST /v1/auth/sign-in` | Requires both `{ username, password }`. |
| `POST /v1/auth/change-password` | Authenticated caller's current and new passwords; `204`, then sign in again. |
| `GET /v1/accounts/{id}/local-login` | Requires `read:account`; returns non-secret login state or `null`. |
| `POST /v1/accounts/{id}/local-login` | Requires `write:account`; attaches a username and returns one activation code. Conflicts do not create another account. |
| `PUT /v1/accounts/{id}/local-login` | Requires `write:account`; explicitly enables or disables local login. |
| `POST /v1/accounts/{id}/local-login/reset` | Requires `write:account`; revokes the password and sessions and returns a replacement reset code. |
| `POST /v1/auth/reset-password` | Public code redemption with a new password; `204`, then ordinary sign-in. |

Username creation and sign-in normalize surrounding whitespace and ASCII case,
then require 3–64 ASCII letters, digits, dots, underscores, or hyphens. Uniqueness
is enforced by `(provider, subject)`. Creating local credentials atomically adds
a JWT identity to an account without a login identity. An account that already
has an identity cannot receive another one through this operation.

The login card shows **Enabled**, **Disabled**, or **Awaiting password**, the
username, and reset expiry. Create/reset codes are shown once, with a copy action;
they are not persisted in browser storage or placed in URLs. The administrator
shares the code privately. The recipient opens the instance's login page, selects
**Use an activation or reset code**, and chooses a password. No email address or
SMTP service is assumed. A lost or expired code requires a new administrative
reset. Resetting a disabled login requires explicitly enabling it first.

The API rejects self-disable and administrative self-reset; the owner uses
**Settings → Change password**. These guards prevent the common accidental
self-lockout, but do not establish a privileged, undeletable admin row or promise
that concurrent administrators cannot lock one another out. The operator recovery
path below restores an existing login. Permission administration remains a
separate explicit operation; recovery does not grant new permissions.

### Storage and security contract

```text
local_credential(
  account_id UUID primary key references account(id) on delete cascade,
  identity_id UUID unique not null references auth_identity(id) on delete cascade,
  password_hash TEXT null,
  enabled BOOLEAN not null default true,
  revision UUID not null,
  reset_hash TEXT unique null,
  reset_expires_at TIMESTAMP null,
  created_at TIMESTAMP not null,
  updated_at TIMESTAMP null
)
```

A null password hash means the account must activate/reset before password login.
Repository hydration verifies that the identity belongs to that account and uses
the JWT provider. Credential attachment and owner setup are transactional. All
credential changes compare the revision read before password verification or
reset issuance with the current revision, preventing a concurrent operation from
restoring an obsolete password or consuming a code twice.

- Argon2id uses a random salt and the PHC format, with 19 MiB memory, two iterations,
  and one lane. New passwords require 15 Unicode scalar values and accept at most
  1024 UTF-8 bytes.
- Each process admits at most 32 password jobs, running at most four blocking
  workers. Sign-in allows ten attempts per normalized username in a rolling
  minute, cleared after successful authentication. The in-memory table is bounded
  to 4096 active usernames; saturation and excess attempts return `429`. Replicas
  have independent counters, so an internet-facing ingress should also enforce
  deployment-wide limits. This is temporary throttling, not persistent account
  disabling.
- Missing, disabled, unactivated, and incorrect-password logins return the same
  `401` response. Missing/disabled credentials perform dummy hash verification.
- Activation/reset codes contain 32 random bytes, encoded as base64url without
  padding; only their SHA-256 digest is stored. They expire after 30 minutes and
  are checked before password hashing. Redemption, password change, and disable
  remove the outstanding grant; issuing a new grant invalidates the previous one.
- Passwords, hashes, JWTs, and reset codes must stay out of request tracing and
  application logs. The only deliberate secret outputs are the reset response,
  its on-screen code, and the operator recovery command's terminal output.

These hashing and reset choices follow the
[OWASP password storage guidance](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
and [OWASP reset-token guidance](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html).
Passwords authenticate a SwissKnife account; they do not derive or recover wallet
private keys, unlock a Lightning node, or replace a wallet backup.

### Schema and first-owner setup

The schema migration creates the local-credential table, unique constraints, and
foreign keys on PostgreSQL and SQLite. Owner setup creates the first local login
as `admin`; all users supply their username when signing in.

Owner setup claims `local_auth_initialized` atomically with account creation.
The marker survives account deletion and every credential reset or disable.
Public sign-up therefore never reopens just because there are no usable local
logins. OAuth2 deployments do not create local credentials from their identities.

### Deployment experience and recovery

| Deployment | Login, management, and recovery experience |
| --- | --- |
| Umbrel | Keep first-run owner setup inside SwissKnife. Household members use separate usernames in the responsive dashboard. Umbrel host access alone does not silently authenticate a SwissKnife account. An operator can run targeted recovery in the backend container. |
| Desktop | A future native wrapper uses the same backend APIs and account IDs. Local or remote instance selection determines the credential namespace. An OS unlock or biometrics feature must not create a second password store or change account ownership. The current dashboard works at desktop sizes. |
| Mobile | The responsive login, code-entry form, and account detail card support narrow screens and password-manager input. A companion client connects to an existing instance and uses its credentials; it cannot reset remote credentials from the device alone. No native mobile app is introduced by this slice. |
| Self-hosted | Both bundled/static and separately hosted dashboards use these APIs. The same database and JWT secret must be shared across backend replicas. Administrators manage users in Account Details; operators can recover an existing local login through database access. |

If no administrator can sign in, an operator with access to the deployment's
configuration and database can recover a specific existing account:

1. Stop the backend replicas and back up the database.
2. Run the same backend binary, with the same configuration/environment:
   `swissknife recover-local-login <account-uuid>`.
3. The command enables that account's existing local login, revokes its password,
   sessions, and old reset grant, and prints a new 30-minute code. It requires
   no Lightning-provider connection and exposes no recovery HTTP endpoint.
4. Restart the backend, open `/reset-password` on that instance, enter the code,
   and choose a password. Sign in with the existing username.

The command does not create an identity, select a hard-coded admin account, grant
permissions, revoke API keys, or delete wallets. If permissions were removed,
another authorized account or an explicit operator permission repair is still
required. Do not delete the initialization marker to regain access.

## Verification and consequences

The implementation is covered through public API tests for independent local
accounts, permissions, password change, disable/enable, reset-code replay and
concurrent redemption, username reuse after deletion, and operator recovery.
Persistence tests exercise the schema on both supported engines and preserve the
production Auth0 account migration coverage. Dashboard tests cover code redemption,
validation, and failure recovery.

JWT authorization now requires a credential lookup, trading stateless local
sessions for prompt revocation. API-key grants remain independent and must be
revoked separately when removing all access. Email reset, MFA/passkeys, username
renaming, cross-provider identity linking, and native desktop/mobile packaging
remain separate product work.
