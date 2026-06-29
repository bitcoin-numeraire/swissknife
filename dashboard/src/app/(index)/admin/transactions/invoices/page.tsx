import { appTitle } from 'src/utils/format-string';

import { AdminInvoiceDetailsRouteView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoice details'),
};

export default function AdminTransactionInvoicePage() {
  return <AdminInvoiceDetailsRouteView />;
}
