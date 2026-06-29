'use client';

import { useRef, useState, useEffect, useCallback } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, usePathname } from 'src/routes/hooks';

import { CONFIG } from 'src/global-config';
import { setupCheck } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

import { useAuthContext } from '../hooks';
import { clearSession } from '../context/jwt';
import { isSameRoutePath } from './setup-route-utils';

// ----------------------------------------------------------------------

type AuthGuardProps = {
  children: React.ReactNode;
};

export function AuthGuard({ children }: AuthGuardProps) {
  const router = useRouter();
  const pathname = usePathname();
  const { authenticated, loading } = useAuthContext();
  const [isChecking, setIsChecking] = useState(true);
  const lastRedirect = useRef<string | null>(null);

  const createRedirectPath = (currentPath: string) => {
    const queryString = new URLSearchParams({ returnTo: pathname }).toString();
    return `${currentPath}?${queryString}`;
  };

  const resetSetupCache = () => {
    localStorage.removeItem(ONBOARDING_COMPLETE_STORAGE_KEY);

    if (CONFIG.auth.method === 'jwt') {
      clearSession();
    }
  };

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

  const checkPermissions = async (): Promise<void> => {
    if (loading) {
      return;
    }

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
    } catch {
      // Let the protected page surface API connectivity errors after the auth check.
    }

    if (!authenticated) {
      const redirectPath = createRedirectPath(paths.auth.login);

      replaceOnce(redirectPath);

      return;
    }

    setIsChecking(false);
  };

  useEffect(() => {
    checkPermissions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [authenticated, loading]);

  if (isChecking) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
