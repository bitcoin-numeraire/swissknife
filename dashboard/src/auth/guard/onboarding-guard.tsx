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
    const onboardingComplete = localStorage.getItem(ONBOARDING_COMPLETE_STORAGE_KEY);

    if (onboardingComplete === 'true') {
      isChecking.onFalse();
      return;
    }

    (async () => {
      try {
        const { data } = await setupCheck<true>();
        if (data.welcome_complete && data.sign_up_complete) {
          localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');
          router.replace(paths.auth.login);
          return;
        }

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
