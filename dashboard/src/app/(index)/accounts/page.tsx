import { appTitle } from 'src/utils/format-string';

import { WalletListView } from 'src/sections/wallet/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Accounts'),
};

export default function AccountsPage() {
  return <WalletListView />;
}
