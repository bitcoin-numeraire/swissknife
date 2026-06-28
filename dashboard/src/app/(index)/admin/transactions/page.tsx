import { appTitle } from 'src/utils/format-string';

import { AdminTransactionsView } from 'src/sections/activity/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Transactions Management'),
};

export default function AdminTransactionsPage() {
  return <AdminTransactionsView />;
}
