'use client';

import { useSearchParams } from 'next/navigation';

import { InvoiceListView } from './invoice-list-view';
import { InvoiceDetailsView } from './invoice-details-view';

// ----------------------------------------------------------------------

export function InvoicesView() {
  const params = useSearchParams();
  const id = params.get('id');
  const walletId = params.get('wallet_id') ?? undefined;

  return id ? <InvoiceDetailsView walletId={walletId} id={id} /> : <InvoiceListView />;
}
