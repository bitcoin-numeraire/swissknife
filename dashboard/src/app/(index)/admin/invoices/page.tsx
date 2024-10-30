import { appTitle } from 'src/utils/format-string';

import { AdminInvoiceListView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoices Management'),
};

export default function AdminInvoiceListPage() {
  return <AdminInvoiceListView />;
}
