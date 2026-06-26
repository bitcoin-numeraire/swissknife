import { appTitle } from 'src/utils/format-string';

import { LegacyActivityRedirect } from 'src/sections/transaction/legacy-activity-redirect';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payments Management'),
};

export default function AdminPaymentListPage() {
  return <LegacyActivityRedirect kind="payment" scope="admin" />;
}
