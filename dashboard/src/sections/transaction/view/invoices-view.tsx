'use client';

import { useSearchParams } from 'next/navigation';

import { InvoiceListView } from './invoice-list-view';
import { InvoiceDetailsView } from './invoice-details-view';

// ----------------------------------------------------------------------

export function InvoicesView() {
  const id = useSearchParams().get('id');

  return id ? <InvoiceDetailsView id={id} /> : <InvoiceListView />;
}
