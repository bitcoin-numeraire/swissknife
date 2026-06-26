'use client';

import React, { useRef, useState, useEffect, useCallback } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, usePathname } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { setupCheck } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

import { useAuthContext } from '../hooks';
import { clearSession } from '../context/jwt';
import { isSameRoutePath } from './setup-route-utils';

function resetSetupCache() {
  localStorage.removeItem(ONBOARDING_COMPLETE_STORAGE_KEY);

  if (CONFIG.auth.method === 'jwt') {
    clearSession();
  }
}

export function OnboardingGuard({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const { authenticated, loading } = useAuthContext();
  const [isChecking, setIsChecking] = useState(true);
  const lastRedirect = useRef<string | null>(null);

  const replaceOnce = useCallback((path: string) => {
    if (isSameRoutePath(pathname, path)) {
      return;
    }

    if (lastRedirect.current === path) {
      return;
    }

    lastRedirect.current = path;
    router.replace(path);
  }, [pathname, router]);

  useEffect(() => {
    if (loading) {
      return undefined;
    }

    let active = true;
    const isWelcomeRoute = isSameRoutePath(pathname, paths.onboarding.welcome);
    const isSignUpRoute = isSameRoutePath(pathname, paths.auth.signUp);

    (async () => {
      try {
        const { data } = await setupCheck<true>();

        if (!data.welcome_complete) {
          resetSetupCache();

          if (isWelcomeRoute) {
            if (active) {
              setIsChecking(false);
            }
            return;
          }

          replaceOnce(paths.onboarding.welcome);
          return;
        }

        if (CONFIG.auth.method === 'jwt' && !data.sign_up_complete) {
          resetSetupCache();

          if (isSignUpRoute) {
            if (active) {
              setIsChecking(false);
            }
            return;
          }

          replaceOnce(paths.auth.signUp);
          return;
        }

        if (data.welcome_complete && (CONFIG.auth.method !== 'jwt' || data.sign_up_complete)) {
          localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');
          replaceOnce(authenticated ? paths.wallet.root : paths.auth.login);
          return;
        }

        if (active) {
          setIsChecking(false);
        }
      } catch (err) {
        handleActionError(err);

        if (active) {
          setIsChecking(false);
        }
      }
    })();

    return () => {
      active = false;
    };
  }, [authenticated, loading, pathname, replaceOnce]);

  if (isChecking) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
