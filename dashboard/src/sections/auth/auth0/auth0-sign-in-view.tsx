'use client';

import { useEffect } from 'react';
import { useAuth0 } from '@auth0/auth0-react';

import { useSearchParams } from 'src/routes/hooks';

import { CONFIG } from 'src/config-global';

// ----------------------------------------------------------------------

export function Auth0SignInView() {
  const { loginWithRedirect } = useAuth0();
  const searchParams = useSearchParams();
  const returnTo = searchParams.get('returnTo');

  useEffect(() => {
    const redirectToLogin = async () => {
      try {
        await loginWithRedirect({ appState: { returnTo: returnTo || CONFIG.auth.redirectPath } });
      } catch (error) {
        console.error(error);
      }
    };

    redirectToLogin();
  }, [loginWithRedirect, returnTo]);

  return null; // Render nothing since the user is being redirected
}
