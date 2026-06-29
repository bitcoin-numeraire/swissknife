import { appTitle } from 'src/utils/format-string';

import { AdminPaymentDetailsRouteView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payment details'),
};

export default function AdminTransactionPaymentPage() {
  return <AdminPaymentDetailsRouteView />;
}
