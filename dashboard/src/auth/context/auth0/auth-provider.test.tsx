import { useContext } from 'react';
import { it, vi, expect } from 'vitest';
import { act, render, screen } from '@testing-library/react';

import { AuthContext } from '../auth-context';
import { AuthProvider } from './auth-provider';

const mocks = vi.hoisted(() => ({
  getAccessTokenSilently: vi.fn(),
  loginWithRedirect: vi.fn(),
  registerInterceptor: vi.fn(),
}));

vi.mock('@auth0/auth0-react', () => ({
  Auth0Provider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useAuth0: () => ({
    user: { sub: 'google-oauth2|test', name: 'Test account' },
    isLoading: false,
    isAuthenticated: true,
    getAccessTokenSilently: mocks.getAccessTokenSilently,
    loginWithRedirect: mocks.loginWithRedirect,
  }),
}));

vi.mock('src/global-config', () => ({
  CONFIG: {
    auth0: {
      domain: 'auth.example.test',
      clientId: 'test-client',
      callbackUrl: 'https://example.test/login/callback',
      audience: 'https://api.example.test',
    },
  },
}));

vi.mock('jwt-decode', () => ({ jwtDecode: () => ({ permissions: ['read:wallet'] }) }));
vi.mock('src/lib/swissknife/client.gen', () => ({
  client: { interceptors: { request: { use: mocks.registerInterceptor } } },
}));

function SessionState() {
  const auth = useContext(AuthContext);

  return (
    <>
      <span data-testid="loading">{String(auth?.loading)}</span>
      <span data-testid="authenticated">{String(auth?.authenticated)}</span>
      <span data-testid="access-token">{auth?.user?.accessToken}</span>
    </>
  );
}

it('waits for the access token before exposing an authenticated session', async () => {
  let resolveToken!: (token: string) => void;
  mocks.getAccessTokenSilently.mockReturnValue(
    new Promise<string>((resolve) => {
      resolveToken = resolve;
    })
  );

  render(
    <AuthProvider>
      <SessionState />
    </AuthProvider>
  );

  expect(screen.getByTestId('loading')).toHaveTextContent('true');
  expect(screen.getByTestId('authenticated')).toHaveTextContent('false');

  await act(async () => {
    resolveToken('signed-access-token');
  });

  expect(screen.getByTestId('loading')).toHaveTextContent('false');
  expect(screen.getByTestId('authenticated')).toHaveTextContent('true');
  expect(screen.getByTestId('access-token')).toHaveTextContent('signed-access-token');
  expect(mocks.registerInterceptor).toHaveBeenCalledOnce();
});
