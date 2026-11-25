'use client';

import { useSearchParams } from 'next/navigation';

import { PaymentListView } from './payment-list-view';
import { PaymentDetailsView } from './payment-details-view';

// ----------------------------------------------------------------------

export function PaymentsView() {
  const params = useSearchParams();
  const id = params.get('id');
  const walletId = params.get('wallet_id') ?? undefined;

  return id ? <PaymentDetailsView walletId={walletId} id={id} /> : <PaymentListView />;
}
