'use client';

import { useSearchParams } from 'next/navigation';

import { AdminInvoiceListView } from './admin-invoice-list-view';
import { AdminInvoiceDetailsView } from './admin-invoice-details-view';

// ----------------------------------------------------------------------

export function AdminInvoicesView() {
  const id = useSearchParams().get('id');

  return id ? <AdminInvoiceDetailsView id={id} /> : <AdminInvoiceListView />;
}
