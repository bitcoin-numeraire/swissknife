'use client';

import { useRef, useState, useEffect, useCallback } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, usePathname, useSearchParams } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { setupCheck } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

import { useAuthContext } from '../hooks';
import { clearSession } from '../context/jwt';
import { getSafeReturnTo, isSameRoutePath } from './setup-route-utils';

// ----------------------------------------------------------------------

type GuestGuardProps = {
  children: React.ReactNode;
};

function resetSetupCache() {
  localStorage.removeItem(ONBOARDING_COMPLETE_STORAGE_KEY);

  if (CONFIG.auth.method === 'jwt') {
    clearSession();
  }
}

export function GuestGuard({ children }: GuestGuardProps) {
  const router = useRouter();
  const pathname = usePathname();
  const lastRedirect = useRef<string | null>(null);

  const searchParams = useSearchParams();
  const returnTo = getSafeReturnTo(searchParams.get('returnTo'), CONFIG.auth.redirectPath);

  const { loading, authenticated } = useAuthContext();

  const [isChecking, setIsChecking] = useState(true);

  const replaceOnce = useCallback(
    (path: string) => {
      if (isSameRoutePath(pathname, path)) {
        return;
      }

      if (lastRedirect.current === path) {
        return;
      }

      lastRedirect.current = path;
      router.replace(path);
    },
    [pathname, router]
  );

  useEffect(() => {
    if (loading) {
      return undefined;
    }

    let active = true;

    (async () => {
      try {
        const { data } = await setupCheck<true>();
        if (!data.welcome_complete) {
          resetSetupCache();
          replaceOnce(paths.onboarding.welcome);
          return;
        }

        if (CONFIG.auth.method === 'jwt' && !data.sign_up_complete) {
          resetSetupCache();
          replaceOnce(paths.auth.signUp);
          return;
        }

        localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');

        if (authenticated) {
          replaceOnce(returnTo);
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
  }, [authenticated, loading, replaceOnce, returnTo]);

  if (isChecking) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
