'use client';

import type { AppState } from '@auth0/auth0-react';
import type { DecodedToken } from 'src/auth/types';

import { jwtDecode } from 'jwt-decode';
import { useMemo, useState, useEffect, useCallback } from 'react';
import {
  useAuth0,
  Auth0Provider,
  AuthenticationError,
  MissingRefreshTokenError,
} from '@auth0/auth0-react';

import { CONFIG } from 'src/global-config';
import { client } from 'src/lib/swissknife';

import { AuthContext } from '../auth-context';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export function AuthProvider({ children }: Props) {
  const { domain, clientId, callbackUrl, audience } = CONFIG.auth0;

  const onRedirectCallback = useCallback((appState?: AppState) => {
    window.location.replace(appState?.returnTo || window.location.pathname);
  }, []);

  if (!(domain && clientId && callbackUrl)) {
    return null;
  }

  return (
    <Auth0Provider
      domain={domain}
      clientId={clientId}
      authorizationParams={{ redirect_uri: callbackUrl, audience }}
      useRefreshTokens
      onRedirectCallback={onRedirectCallback}
      cacheLocation="localstorage"
    >
      <AuthProviderContainer>{children}</AuthProviderContainer>
    </Auth0Provider>
  );
}

// ----------------------------------------------------------------------

function AuthProviderContainer({ children }: Props) {
  const { user, isLoading, isAuthenticated, getAccessTokenSilently, loginWithRedirect, logout } =
    useAuth0();
  const { audience } = CONFIG.auth0;

  const [accessToken, setAccessToken] = useState<string | null>(null);
  const [permissions, setPermissions] = useState<string[]>([]);

  const getAccessToken = useCallback(async () => {
    try {
      const token = await getAccessTokenSilently({ authorizationParams: { audience } });
      setAccessToken(token);
      setPermissions(jwtDecode<DecodedToken>(token).permissions || []);

      client.interceptors.request.use(async (request) => {
        try {
          const t = await getAccessTokenSilently({ authorizationParams: { audience } });
          request.headers.set('Authorization', `Bearer ${t}`);
        } catch (err: unknown) {
          console.error('Token expired or missing, redirecting to login', err);
          loginWithRedirect();
        }
        return request;
      });
    } catch (err: unknown) {
      console.error('Failed to get token:', err);

      setAccessToken(null);
      setPermissions([]);

      if (err instanceof MissingRefreshTokenError) {
        loginWithRedirect();
      } else if (err instanceof AuthenticationError && err.error === 'invalid_grant') {
        loginWithRedirect();
      } else {
        logout();
      }
    }
  }, [getAccessTokenSilently, audience, loginWithRedirect, logout]);

  useEffect(() => {
    getAccessToken();
  }, [getAccessToken]);

  // ----------------------------------------------------------------------

  const checkAuthenticated = isAuthenticated ? 'authenticated' : 'unauthenticated';

  const status = isLoading ? 'loading' : checkAuthenticated;

  const memoizedValue = useMemo(
    () => ({
      user: user
        ? {
            ...user,
            id: user?.sub,
            accessToken,
            displayName: user?.name,
            photoURL: user?.picture,
            permissions,
          }
        : null,
      loading: status === 'loading',
      authenticated: status === 'authenticated',
      unauthenticated: status === 'unauthenticated',
    }),
    [accessToken, status, user, permissions]
  );

  return <AuthContext.Provider value={memoizedValue}>{children}</AuthContext.Provider>;
}
