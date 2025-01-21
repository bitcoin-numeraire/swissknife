import { appTitle } from 'src/utils/format-string';

import { InvoicesView } from 'src/sections/transaction/view/invoices-view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoices'),
};

export default function InvoiceListPage() {
  return <InvoicesView />;
}
