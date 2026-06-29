'use client';

import { useEffect } from 'react';

import { paths } from 'src/routes/paths';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { AdminInvoiceDetailsView } from './admin-invoice-details-view';

// ----------------------------------------------------------------------

export function AdminInvoiceDetailsRouteView() {
  const router = useRouter();
  const id = useSearchParams().get('id');

  useEffect(() => {
    if (!id) {
      router.replace(paths.admin.transactionList('invoice'));
    }
  }, [id, router]);

  return id ? <AdminInvoiceDetailsView id={id} /> : null;
}
