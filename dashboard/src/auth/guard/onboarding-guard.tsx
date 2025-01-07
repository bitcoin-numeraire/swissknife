'use client';

import React, { useEffect } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { setupCheck } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

export function OnboardingGuard({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const isChecking = useBoolean(true);

  useEffect(() => {
    const localValue = localStorage.getItem(ONBOARDING_COMPLETE_STORAGE_KEY);

    if (localValue === 'true') {
      isChecking.onFalse();
      return;
    }

    (async () => {
      try {
        const { data } = await setupCheck<true>();
        if (data.welcome_complete) {
          router.replace(paths.onboarding.welcome);
          return;
        }
        if (!data.setup_complete) {
          router.replace(paths.onboarding.setup.root);
          return;
        }

        localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');
        isChecking.onFalse();
      } catch (err) {
        handleActionError(err);
      }
    })();
  }, [isChecking, router]);

  if (isChecking.value) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
