'use client';

import { useEffect } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { AdminPaymentDetailsView } from './admin-payment-details-view';

// ----------------------------------------------------------------------

export function AdminPaymentDetailsRouteView() {
  const router = useRouter();
  const id = useSearchParams().get('id');

  useEffect(() => {
    if (!id) {
      router.replace(paths.admin.transactionList('payment'));
    }
  }, [id, router]);

  return id ? <AdminPaymentDetailsView id={id} /> : null;
}
