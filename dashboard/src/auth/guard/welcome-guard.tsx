'use client';

import React, { useEffect } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import { handleActionError } from 'src/utils/errors';

import { SimpleLayout } from 'src/layouts/simple';
import { setupCheck, markWelcomeComplete } from 'src/lib/swissknife';

import { SplashScreen } from 'src/components/loading-screen';

import { WelcomeView } from 'src/sections/welcome/view/welcome-view';

const WELCOME_KEY = 'welcomeCompleted';

export default function WelcomeGuard({ children }: { children: React.ReactNode }) {
  const loading = useBoolean(true);
  const welcome = useBoolean(false);

  useEffect(() => {
    const localValue = localStorage.getItem(WELCOME_KEY);

    if (localValue === 'true') {
      loading.onFalse();
      return;
    }

    (async () => {
      try {
        const { data } = await setupCheck<true>();
        if (data.welcome_complete) {
          localStorage.setItem(WELCOME_KEY, 'true');
          loading.onFalse();
        } else {
          welcome.onTrue();
          loading.onFalse();
        }
      } catch (err) {
        handleActionError(err);
      }
    })();
  }, [loading, welcome]);

  const handleWelcomeComplete = async () => {
    try {
      await markWelcomeComplete();
      localStorage.setItem(WELCOME_KEY, 'true');
    } catch (err) {
      handleActionError(err);
    } finally {
      welcome.onFalse();
    }
  };

  if (loading.value) {
    return <SplashScreen />;
  }

  if (welcome.value) {
    return (
      <SimpleLayout>
        <WelcomeView onComplete={handleWelcomeComplete} />
      </SimpleLayout>
    );
  }

  return <>{children}</>;
}
