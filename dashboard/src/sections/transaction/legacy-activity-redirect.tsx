'use client';

import { useMemo, useEffect } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { SplashScreen } from 'src/components/loading-screen';

// ----------------------------------------------------------------------

type Props = {
  kind: 'payment' | 'invoice';
  scope?: 'wallet' | 'admin';
};

export function LegacyActivityRedirect({ kind, scope = 'wallet' }: Props) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const id = searchParams.get('id') || undefined;

  const href = useMemo(() => {
    if (kind === 'invoice') {
      return id ? paths.activityInvoice(id, scope) : paths.activityList('invoice', scope);
    }

    return id ? paths.activityPayment(id, scope) : paths.activityList('payment', scope);
  }, [id, kind, scope]);

  useEffect(() => {
    router.replace(href);
  }, [href, router]);

  return <SplashScreen />;
}
