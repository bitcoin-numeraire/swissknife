This file provides guidance to AI Agents when working with code in this repository.

## Development Commands

```bash
yarn install                   # Install dependencies (Yarn 4 required)
yarn dev                       # Start dev server on port 8080
yarn build                     # Production build
yarn test                      # Run Vitest tests
yarn test:e2e                  # Run Playwright tests
yarn lint                      # Run ESLint
yarn lint:fix                  # Fix ESLint errors
yarn fm:fix                    # Format with Prettier
yarn fix:all                   # Run both lint:fix and fm:fix
yarn openapi-ts                # Regenerate API client from OpenAPI spec
yarn typecheck                 # Type-check once
yarn tsc:watch                 # Type-check in watch mode
```

**Node 24.x and Yarn 4 (via corepack) required** (see `engines` / `packageManager` in package.json).

## Architecture

### App Router Structure (`src/app/`)

Uses the Next.js 16 App Router (React 19, MUI v9) with route groups:
- `(index)/` - Protected routes wrapped with AuthGuard
- Auth pages at root level: `/login`, `/sign-up`, `/reset-password`, `/verify`

Main route sections:
- `/wallet/*` - End-user wallet views (dashboard, payments, invoices, contacts)
- `/admin/*` - Administrative accounts, wallets, transactions, addresses, and API keys
- `/settings` - User settings
- `/welcome` - Onboarding flow

### Code Organization

**`src/sections/`** - Page-specific components organized by feature. Each section typically has a `view.tsx` as the main entry point.

**`src/components/`** - Reusable UI components (buttons, forms, tables, modals).

**`src/layouts/`** - Page layout wrappers (dashboard layout with sidebar, auth layout for centered forms).

**`src/actions/`** - Data-fetching hooks using SWR. Server actions marked with `'use server'`.

**`src/auth/`** - Pluggable authentication system supporting JWT (default), Auth0, and Supabase. Selection via `NEXT_PUBLIC_AUTH_METHOD` env var.

**`src/lib/swissknife/`** - Auto-generated API client from OpenAPI spec. **Do not edit manually** - regenerate with `yarn openapi-ts`.

**`src/routes/paths.ts`** - Centralized route path definitions.

### Data Flow Pattern

1. Page imports section view from `src/sections/`
2. Section uses SWR hooks from `src/actions/` for data fetching
3. Forms use react-hook-form with Zod schemas from generated API client
4. API calls go through `src/lib/swissknife/sdk.gen.ts`

### State Management

- **React Context** for auth state and UI settings (theme, language)
- **SWR** for all server data (caching, revalidation, mutations)
- **react-hook-form** for form state with Zod validation
- No Redux/Zustand - intentionally lightweight

### API Client

Auto-generated (hey-api / `@hey-api/openapi-ts`) from the backend's OpenAPI spec checked in at `src/lib/openapi.json`:
- `client.gen.ts` - configured fetch client instance (vendored; import `client` from here)
- `sdk.gen.ts` - API endpoint functions
- `types.gen.ts` - TypeScript types
- `zod.gen.ts` - Zod validation schemas
- `transformers.gen.ts` - response transformers (e.g. date strings → `Date`)

Generator config lives in `openapi-ts.config.mjs`. The whole `src/lib/swissknife/` dir is **generated — do not edit by hand**, and is excluded from ESLint. Base URL / auth interceptors are set in `src/global-config.ts` and `src/auth/context/*` (base URL from `NEXT_PUBLIC_SERVER_URL`).

Note: `@hey-api/transformers` is configured with `bigInt: false`, so `int64` fields (e.g. `amount_msat`) are emitted as `number`. See issue #274 for the bigint precision follow-up.

### Regenerating the OpenAPI spec + client

The spec is produced from the backend's **utoipa** annotations (`src/application/docs/openapi.rs::merged_openapi()`, also served live at `/docs`) — it is NOT hand-written. Run this whenever the backend API changes (new/changed routes, DTOs, enums):

```bash
# From the repo root (swissknife/) — regenerates BOTH the spec and the client:
make openapi
```

That target runs two steps:
1. `cargo test --quiet dump_openapi_spec -- --ignored` — writes the current backend spec to `dashboard/src/lib/openapi.json`. This is an `#[ignore]`d generation test in `src/application/docs/openapi.rs`, so it is skipped during normal `make test` and only runs on demand.
2. `cd dashboard && yarn openapi-ts` — regenerates `src/lib/swissknife/` from that spec.

The spec version tracks the backend crate version (`CARGO_PKG_VERSION`). Step 1 compiles the backend, so the Rust toolchain is required (see `swissknife/AGENTS.md`). To only regenerate the client from an already-updated spec, run `yarn openapi-ts` in `dashboard/`.

### Authentication Guards

Located in `src/auth/guard/`:
- `AuthGuard` - Redirects unauthenticated users to login
- `GuestGuard` - Redirects authenticated users away from auth pages
- `RoleBasedGuard` - Permission checking from JWT claims
- `OnboardingGuard` - Enforces welcome flow completion

### Build Modes

**Standalone** (default): Independent Next.js server connecting to backend via `NEXT_PUBLIC_SERVER_URL`.

**Static Export**: Set `BUILD_STATIC_EXPORT=true` for bundling with Rust backend. Outputs static files served by backend.

### Key Environment Variables

```bash
NEXT_PUBLIC_SERVER_URL        # Backend API URL (empty = same origin)
NEXT_PUBLIC_AUTH_METHOD       # 'jwt' | 'auth0' | 'supabase'
NEXT_PUBLIC_APPNAME           # App display name
BUILD_STATIC_EXPORT           # 'true' for static build
```

See `.env.example` for full list including Auth0/Supabase config.

## Code Review Guidelines

### General
- Run `yarn lint`, `yarn typecheck`, `yarn test`, and `yarn build` before committing
- Format code with `yarn fix:all`

### React/Next.js
- Use Server Components by default; only add `'use client'` when necessary
- Avoid `useEffect` for data fetching; use SWR hooks from `src/actions/`
- Ensure proper loading and error states for async operations
- Check for missing `key` props in lists
- Avoid inline styles; use MUI's `sx` prop or theme system

### Data Fetching
- Use existing SWR hooks or create new ones following the pattern in `src/actions/`
- Handle loading, error, and empty states in UI
- Use optimistic updates for better UX where appropriate
- Never call API endpoints directly in components; go through the SDK

### Type Safety
- Do not use `any` type; use proper types from `types.gen.ts`
- Use Zod schemas from `zod.gen.ts` for form validation
- Ensure API response types match what the backend returns

### Security
- Never expose sensitive data in client-side code
- Use auth guards appropriately for protected routes
- Validate and sanitize user input before API calls
- Check that authentication tokens are handled securely
