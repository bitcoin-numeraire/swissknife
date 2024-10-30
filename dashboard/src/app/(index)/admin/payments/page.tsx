import { appTitle } from 'src/utils/format-string';

import { AdminPaymentListView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payments Management'),
};

export default function AdminPaymentListPage() {
  return <AdminPaymentListView />;
}
