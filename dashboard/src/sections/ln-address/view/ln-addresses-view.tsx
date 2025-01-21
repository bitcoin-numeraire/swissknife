'use client';

import { useSearchParams } from 'next/navigation';

import { LnAddressListView } from './ln-address-list-view';
import { AdminLnAddressDetailsView } from './admin-ln-address-details-view';

// ----------------------------------------------------------------------

export function LnAddressesView() {
  const id = useSearchParams().get('id');

  return id ? <AdminLnAddressDetailsView id={id} /> : <LnAddressListView />;
}
