import { appTitle } from 'src/utils/format-string';

import { InvoiceListView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoices'),
};

export default function InvoiceListPage() {
  return <InvoiceListView />;
}
