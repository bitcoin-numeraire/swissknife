'use client';

import { useBoolean } from 'minimal-shared/hooks';
import React, { useRef, useEffect, useCallback } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, usePathname } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { setupCheck } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

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
  const isChecking = useBoolean(true);
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
    const isWelcomeRoute = isSameRoutePath(pathname, paths.onboarding.welcome);
    const isSignUpRoute = isSameRoutePath(pathname, paths.auth.signUp);

    (async () => {
      try {
        const { data } = await setupCheck<true>();

        if (!data.welcome_complete) {
          resetSetupCache();

          if (isWelcomeRoute) {
            isChecking.onFalse();
            return;
          }

          replaceOnce(paths.onboarding.welcome);
          return;
        }

        if (CONFIG.auth.method === 'jwt' && !data.sign_up_complete) {
          resetSetupCache();

          if (isSignUpRoute) {
            isChecking.onFalse();
            return;
          }

          replaceOnce(paths.auth.signUp);
          return;
        }

        if (data.welcome_complete && (CONFIG.auth.method !== 'jwt' || data.sign_up_complete)) {
          localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');
          replaceOnce(paths.auth.login);
          return;
        }

        isChecking.onFalse();
      } catch (err) {
        handleActionError(err);
      }
    })();
  }, [isChecking, pathname, replaceOnce]);

  if (isChecking.value) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
