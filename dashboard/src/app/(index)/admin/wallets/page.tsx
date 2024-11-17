import { appTitle } from 'src/utils/format-string';

import { WalletListView } from 'src/sections/wallet/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Wallets Management'),
};

export default function AdminWalletListPage() {
  return <WalletListView />;
}
