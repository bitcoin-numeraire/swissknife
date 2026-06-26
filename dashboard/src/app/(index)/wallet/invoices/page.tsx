import { appTitle } from 'src/utils/format-string';

import { LegacyActivityRedirect } from 'src/sections/transaction/legacy-activity-redirect';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoices'),
};

export default function InvoiceListPage() {
  return <LegacyActivityRedirect kind="invoice" />;
}
