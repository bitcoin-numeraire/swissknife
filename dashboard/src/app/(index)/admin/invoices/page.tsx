import { appTitle } from 'src/utils/format-string';

import { LegacyActivityRedirect } from 'src/sections/transaction/legacy-activity-redirect';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoices Management'),
};

export default function AdminInvoiceListPage() {
  return <LegacyActivityRedirect kind="invoice" scope="admin" />;
}
