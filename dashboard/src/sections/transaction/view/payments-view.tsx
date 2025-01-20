'use client';

import { useSearchParams } from 'next/navigation';

import { PaymentListView } from './payment-list-view';
import { PaymentDetailsView } from './payment-details-view';

// ----------------------------------------------------------------------

export function PaymentsView() {
  const id = useSearchParams().get('id');

  return id ? <PaymentDetailsView id={id} /> : <PaymentListView />;
}
