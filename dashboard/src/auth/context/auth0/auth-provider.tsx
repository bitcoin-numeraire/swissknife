'use client';

import type { AppState } from '@auth0/auth0-react';
import type { DecodedToken } from 'src/auth/types';

import { jwtDecode } from 'jwt-decode';
import { useAuth0, Auth0Provider } from '@auth0/auth0-react';
import { useMemo, useState, useEffect, useCallback } from 'react';

import { CONFIG } from 'src/global-config';
import { client } from 'src/lib/swissknife/client.gen';

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
      onRedirectCallback={onRedirectCallback}
      cacheLocation="localstorage"
    >
      <AuthProviderContainer>{children}</AuthProviderContainer>
    </Auth0Provider>
  );
}

// ----------------------------------------------------------------------

type ApiSession = {
  subject: string;
  accessToken: string | null;
  permissions: DecodedToken['permissions'];
};

function AuthProviderContainer({ children }: Props) {
  const { user, isLoading, isAuthenticated, getAccessTokenSilently, loginWithRedirect } =
    useAuth0();
  const subject = user?.sub;
  const [session, setSession] = useState<ApiSession | null>(null);

  useEffect(() => {
    if (isLoading || !isAuthenticated || !subject) return undefined;

    let active = true;
    let interceptorId: number | undefined;
    let redirecting = false;

    const getAccessToken = async () => {
      const token = await getAccessTokenSilently();
      if (!token) throw new Error('Auth0 did not return an API access token');
      return token;
    };

    const authorizeRequest = async (request: Request) => {
      try {
        const token = await getAccessToken();
        if (!active) throw new DOMException('Auth0 session ended', 'AbortError');
        request.headers.set('Authorization', `Bearer ${token}`);
        return request;
      } catch (error) {
        if (active && !redirecting) {
          redirecting = true;
          void loginWithRedirect({ appState: { returnTo: window.location.href } }).catch(() => {
            redirecting = false;
            console.error('Could not redirect to Auth0 for sign-in');
          });
        }
        throw error;
      }
    };

    const startSession = async () => {
      try {
        const accessToken = await getAccessToken();
        if (!active) return;

        const { permissions = [] } = jwtDecode<DecodedToken>(accessToken);
        interceptorId = client.interceptors.request.use(authorizeRequest);

        // The guards may start API requests as soon as this session is published.
        setSession({ subject, accessToken, permissions });
      } catch {
        if (active) setSession({ subject, accessToken: null, permissions: [] });
      }
    };

    void startSession();

    return () => {
      active = false;
      if (interceptorId !== undefined) client.interceptors.request.eject(interceptorId);
      setSession(null);
    };
  }, [isLoading, isAuthenticated, subject, getAccessTokenSilently, loginWithRedirect]);

  const currentSession = session?.subject === subject ? session : null;
  const loading = isLoading || Boolean(isAuthenticated && subject && !currentSession);
  const authenticated = !loading && isAuthenticated && Boolean(currentSession?.accessToken);

  const memoizedValue = useMemo(
    () => ({
      user:
        authenticated && user && currentSession
          ? {
              ...user,
              id: user.sub,
              accessToken: currentSession.accessToken,
              displayName: user.name,
              photoURL: user.picture,
              permissions: currentSession.permissions,
            }
          : null,
      loading,
      authenticated,
      unauthenticated: !loading && !authenticated,
    }),
    [authenticated, currentSession, loading, user]
  );

  return <AuthContext.Provider value={memoizedValue}>{children}</AuthContext.Provider>;
}
