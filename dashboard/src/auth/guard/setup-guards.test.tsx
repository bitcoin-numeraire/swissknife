import type { AuthMethod } from 'src/global-config';

import { vi, it, expect, describe, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

import { paths } from 'src/routes/paths';

import { AuthGuard } from './auth-guard';
import { GuestGuard } from './guest-guard';
import { OnboardingGuard } from './onboarding-guard';

const mocks = vi.hoisted(() => ({
  router: { replace: vi.fn() },
  setupCheck: vi.fn(),
  clearSession: vi.fn(),
  handleActionError: vi.fn(),
  state: {
    method: 'auth0' as AuthMethod,
    pathname: '/wallet',
    authenticated: true,
  },
}));

vi.mock('src/global-config', () => ({
  CONFIG: {
    auth: {
      get method() {
        return mocks.state.method;
      },
      redirectPath: '/wallet',
    },
  },
}));

vi.mock('src/routes/hooks', () => ({
  useRouter: () => mocks.router,
  usePathname: () => mocks.state.pathname,
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock('src/lib/swissknife', () => ({ setupCheck: mocks.setupCheck }));
vi.mock('src/utils/errors', () => ({ handleActionError: mocks.handleActionError }));
vi.mock('src/components/loading-screen', () => ({ SplashScreen: () => <span>Loading</span> }));
vi.mock('src/components/settings', () => ({
  ONBOARDING_COMPLETE_STORAGE_KEY: 'onboarding-complete',
}));
vi.mock('../context/jwt', () => ({ clearSession: mocks.clearSession }));
vi.mock('../hooks', () => ({
  useAuthContext: () => ({ authenticated: mocks.state.authenticated, loading: false }),
}));

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  mocks.state.method = 'auth0';
  mocks.state.authenticated = true;
  mocks.setupCheck.mockResolvedValue({
    data: { welcome_complete: false, sign_up_complete: false },
  });
});

async function expectDestination(destination: string | null) {
  if (destination) {
    await waitFor(() => expect(mocks.router.replace).toHaveBeenCalledWith(destination));
  } else {
    expect(await screen.findByText('Page content')).toBeInTheDocument();
    expect(mocks.router.replace).not.toHaveBeenCalled();
  }
}

describe.each([
  {
    name: 'AuthGuard',
    Guard: AuthGuard,
    pathname: paths.wallet.root,
    authenticatedDestination: null,
    unauthenticatedDestination: `${paths.auth.login}?returnTo=%2Fwallet`,
    freshJwtDestination: paths.onboarding.welcome,
  },
  {
    name: 'GuestGuard',
    Guard: GuestGuard,
    pathname: paths.auth.login,
    authenticatedDestination: paths.wallet.root,
    unauthenticatedDestination: null,
    freshJwtDestination: paths.onboarding.welcome,
  },
  {
    name: 'OnboardingGuard',
    Guard: OnboardingGuard,
    pathname: paths.onboarding.welcome,
    authenticatedDestination: paths.wallet.root,
    unauthenticatedDestination: paths.auth.login,
    freshJwtDestination: null,
  },
])(
  '$name',
  ({
    Guard,
    pathname,
    authenticatedDestination,
    unauthenticatedDestination,
    freshJwtDestination,
  }) => {
    beforeEach(() => {
      mocks.state.pathname = pathname;
    });

    it.each<AuthMethod>(['auth0', 'mock-oauth2'])(
      'uses the authenticated %s session without local setup flags',
      async (method) => {
        mocks.state.method = method;

        render(<Guard>Page content</Guard>);

        await expectDestination(authenticatedDestination);
        expect(mocks.setupCheck).not.toHaveBeenCalled();
        expect(mocks.clearSession).not.toHaveBeenCalled();
      }
    );

    it.each<AuthMethod>(['auth0', 'mock-oauth2'])(
      'routes an unauthenticated %s user without local onboarding',
      async (method) => {
        mocks.state.method = method;
        mocks.state.authenticated = false;

        render(<Guard>Page content</Guard>);

        await expectDestination(unauthenticatedDestination);
        expect(mocks.setupCheck).not.toHaveBeenCalled();
      }
    );

    it('still requires the welcome flow for a fresh local JWT installation', async () => {
      mocks.state.method = 'jwt';

      render(<Guard>Page content</Guard>);

      await expectDestination(freshJwtDestination);
      expect(mocks.setupCheck).toHaveBeenCalled();
    });

    it('still requires local sign-up after the welcome flow', async () => {
      mocks.state.method = 'jwt';
      mocks.setupCheck.mockResolvedValue({
        data: { welcome_complete: true, sign_up_complete: false },
      });

      render(<Guard>Page content</Guard>);

      await expectDestination(paths.auth.signUp);
    });

    it('accepts a completed local JWT installation', async () => {
      mocks.state.method = 'jwt';
      mocks.setupCheck.mockResolvedValue({
        data: { welcome_complete: true, sign_up_complete: true },
      });

      render(<Guard>Page content</Guard>);

      await expectDestination(authenticatedDestination);
    });
  }
);
