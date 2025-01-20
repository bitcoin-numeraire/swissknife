import { appTitle } from 'src/utils/format-string';

import { AdminPaymentsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payments Management'),
};

export default function AdminPaymentListPage() {
  return <AdminPaymentsView />;
}
