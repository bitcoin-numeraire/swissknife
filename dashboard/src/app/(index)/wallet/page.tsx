import { appTitle } from 'src/utils/format-string';

import { WalletView } from 'src/sections/wallet/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Wallet'),
};

export default function WalletPage() {
  return <WalletView />;
}
