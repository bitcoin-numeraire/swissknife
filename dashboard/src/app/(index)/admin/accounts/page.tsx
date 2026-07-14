import { appTitle } from 'src/utils/format-string';

import { AccountsView } from 'src/sections/accounts/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Accounts'),
};

export default function AccountsPage() {
  return <AccountsView />;
}
