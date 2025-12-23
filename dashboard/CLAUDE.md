This file provides guidance to AI Agents when working with code in this repository.

## Development Commands

```bash
yarn install                   # Install dependencies (Yarn 4 required)
yarn dev                       # Start dev server on port 8080
yarn build                     # Production build
yarn lint                      # Run ESLint
yarn lint:fix                  # Fix ESLint errors
yarn fm:fix                    # Format with Prettier
yarn fix:all                   # Run both lint:fix and fm:fix
yarn openapi-ts                # Regenerate API client from OpenAPI spec
yarn tsc:watch                 # Type-check in watch mode
```

**Node 20.x required** (see `engines` in package.json).

## Architecture

### App Router Structure (`src/app/`)

Uses Next.js 14 App Router with route groups:
- `(index)/` - Protected routes wrapped with AuthGuard
- Auth pages at root level: `/login`, `/sign-up`, `/reset-password`, `/verify`

Main route sections:
- `/wallet/*` - End-user wallet views (dashboard, payments, invoices, contacts)
- `/admin/*` - Admin views (wallets, api-keys, lightning-node)
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

Auto-generated from backend's OpenAPI spec at `src/lib/openapi.json`:
- `sdk.gen.ts` - API endpoint functions
- `types.gen.ts` - TypeScript types
- `zod.gen.ts` - Zod validation schemas

Client configured in `src/global-config.ts` with base URL from `NEXT_PUBLIC_SERVER_URL`.

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
- Run `yarn lint` and `yarn build` before committing
- Ensure TypeScript has no type errors (`yarn tsc:watch`)
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
