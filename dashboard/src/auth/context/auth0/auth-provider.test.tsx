import { useEffect } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import { it, vi, expect, describe, afterEach, beforeEach } from 'vitest';

import { getAccount } from 'src/lib/swissknife';
import { client } from 'src/lib/swissknife/client.gen';

import { AuthProvider } from './auth-provider';
import { AuthGuard } from '../../guard/auth-guard';

const { auth0, replace } = vi.hoisted(() => ({
  auth0: {
    isLoading: false,
    isAuthenticated: true,
    user: { sub: 'auth0|user' },
    getAccessTokenSilently: vi.fn<() => Promise<string>>(),
    loginWithRedirect: vi.fn<() => Promise<void>>(),
  },
  replace: vi.fn(),
}));

vi.mock('@auth0/auth0-react', () => ({
  Auth0Provider: ({ children }: { children: React.ReactNode }) => children,
  useAuth0: () => auth0,
}));

vi.mock('src/global-config', () => ({
  CONFIG: {
    auth: { method: 'auth0' },
    auth0: {
      domain: 'auth.example.com',
      clientId: 'dashboard',
      callbackUrl: 'https://app.example.com/login/callback',
      audience: 'https://api.example.com',
    },
  },
}));

vi.mock('src/routes/hooks', () => ({
  useRouter: () => ({ replace }),
  usePathname: () => '/wallet',
}));

vi.mock('src/components/loading-screen', () => ({ SplashScreen: () => <p>Loading</p> }));
vi.mock('src/components/settings', () => ({ ONBOARDING_COMPLETE_STORAGE_KEY: 'onboarding' }));

const token = `header.${btoa(JSON.stringify({ permissions: ['read:wallet'] }))}.signature`;
const fetchApi = vi.fn(async (request: Request) => {
  if (new URL(request.url).pathname === '/v1/system/setup') {
    return Response.json({ welcome_complete: true });
  }

  if (!request.headers.has('Authorization')) {
    return Response.json({ message: 'Unauthorized' }, { status: 401 });
  }

  return Response.json({ id: 'account-1', created_at: '2026-11-10T00:00:00Z', wallets: [] });
});

const originalConfig = client.getConfig();

beforeEach(() => {
  vi.resetAllMocks();
  auth0.loginWithRedirect.mockResolvedValue(undefined);
  client.setConfig({
    baseUrl: 'https://api.example.com',
    fetch: (input, init) => fetchApi(new Request(input, init)),
    throwOnError: true,
  });
});

afterEach(() => {
  client.setConfig(originalConfig);
  localStorage.clear();
});

function AccountRequest() {
  useEffect(() => {
    void getAccount().catch(() => {});
  }, []);

  return <p>Wallet dashboard</p>;
}

function renderDashboard() {
  return render(
    <AuthProvider>
      <AuthGuard>
        <AccountRequest />
      </AuthGuard>
    </AuthProvider>
  );
}

describe('Auth0 dashboard session', () => {
  it('waits for the API token before loading the account and uses renewed tokens for later requests', async () => {
    const firstToken = Promise.withResolvers<string>();
    auth0.getAccessTokenSilently.mockReturnValue(firstToken.promise);
    const dashboard = renderDashboard();

    await act(async () => {});
    expect(screen.getByText('Loading')).toBeInTheDocument();
    expect(fetchApi).not.toHaveBeenCalled();

    await act(async () => firstToken.resolve(token));
    await waitFor(() => expect(fetchApi).toHaveBeenCalledTimes(2));
    expect(screen.getByText('Wallet dashboard')).toBeInTheDocument();
    expect(fetchApi.mock.calls.map(([request]) => request.headers.get('Authorization'))).toEqual([
      `Bearer ${token}`,
      `Bearer ${token}`,
    ]);

    auth0.getAccessTokenSilently.mockResolvedValue('renewed-token');
    await getAccount();
    expect(fetchApi.mock.lastCall?.[0].headers.get('Authorization')).toBe('Bearer renewed-token');

    dashboard.unmount();
    await getAccount({ throwOnError: false });
    expect(fetchApi.mock.lastCall?.[0].headers.has('Authorization')).toBe(false);
  });

  it('redirects to sign-in when the initial API token is unavailable', async () => {
    auth0.getAccessTokenSilently.mockRejectedValue(new Error('login_required'));
    renderDashboard();

    await waitFor(() => expect(replace).toHaveBeenCalledWith('/login?returnTo=%2Fwallet'));
    expect(screen.queryByText('Wallet dashboard')).not.toBeInTheDocument();
    expect(fetchApi.mock.calls.map(([request]) => new URL(request.url).pathname)).toEqual([
      '/v1/system/setup',
    ]);
  });

  it('stops an API request when silent renewal fails', async () => {
    auth0.getAccessTokenSilently.mockResolvedValue(token);
    renderDashboard();
    await waitFor(() => expect(fetchApi).toHaveBeenCalledTimes(2));

    const error = new Error('login_required');
    auth0.getAccessTokenSilently.mockRejectedValue(error);
    await expect(getAccount({ throwOnError: true })).rejects.toBe(error);

    expect(auth0.loginWithRedirect).toHaveBeenCalledOnce();
    expect(fetchApi).toHaveBeenCalledTimes(2);
  });
});
