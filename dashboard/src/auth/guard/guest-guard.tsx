'use client';

import { useEffect } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import { paths } from 'src/routes/paths';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { setupCheck } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

import { useAuthContext } from '../hooks';

// ----------------------------------------------------------------------

type GuestGuardProps = {
  children: React.ReactNode;
};

export function GuestGuard({ children }: GuestGuardProps) {
  const router = useRouter();

  const searchParams = useSearchParams();
  const returnTo = searchParams.get('returnTo') || CONFIG.auth.redirectPath;

  const { loading, authenticated } = useAuthContext();

  const isChecking = useBoolean(true);

  useEffect(() => {
    if (loading) {
      return;
    }

    if (authenticated) {
      router.replace(returnTo);
      return;
    }

    if (localStorage.getItem(ONBOARDING_COMPLETE_STORAGE_KEY) === 'true') {
      isChecking.onFalse();
      return;
    }

    (async () => {
      try {
        const { data } = await setupCheck<true>();
        if (!data.welcome_complete) {
          router.replace(paths.onboarding.welcome);
          return;
        }

        if (CONFIG.auth.method === 'jwt' && !data.sign_up_complete) {
          router.replace(paths.auth.signUp);
          return;
        }

        localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');
        isChecking.onFalse();
      } catch (err) {
        handleActionError(err);
      }
    })();
  }, [authenticated, loading, isChecking, returnTo, router]);

  if (isChecking.value) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
