'use client';

import { useSearchParams } from 'next/navigation';

import { AdminPaymentListView } from './admin-payment-list-view';
import { AdminPaymentDetailsView } from './admin-payment-details-view';

// ----------------------------------------------------------------------

export function AdminPaymentsView() {
  const id = useSearchParams().get('id');

  return id ? <AdminPaymentDetailsView id={id} /> : <AdminPaymentListView />;
}
