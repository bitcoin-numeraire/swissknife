'use client';

import { useState, useEffect, useCallback } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, usePathname, useSearchParams } from 'src/routes/hooks';

import { CONFIG } from 'src/config-global';

import { SplashScreen } from 'src/components/loading-screen';

import { useAuthContext } from '../hooks';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export function AuthGuard({ children }: Props) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const { authenticated, loading } = useAuthContext();
  const [isChecking, setIsChecking] = useState<boolean>(true);

  const createQueryString = useCallback(
    (name: string, value: string) => {
      const params = new URLSearchParams(searchParams.toString());
      params.set(name, value);
      return params.toString();
    },
    [searchParams]
  );

  const checkPermissions = useCallback(async (): Promise<void> => {
    if (loading) {
      return;
    }

    if (!authenticated) {
      const { method } = CONFIG.auth;
      const signInPath = {
        jwt: paths.auth.jwt.signIn,
        auth0: paths.auth.auth0.signIn,
        supabase: paths.auth.supabase.signIn,
      }[method];

      const href = `${signInPath}?${createQueryString('returnTo', pathname)}`;
      router.replace(href);
    } else {
      setIsChecking(false);
    }
  }, [authenticated, loading, router, pathname, createQueryString]);

  useEffect(() => {
    checkPermissions();
  }, [checkPermissions]);

  if (isChecking || loading) {
    return <SplashScreen />;
  }

  return <>{children}</>;
}
