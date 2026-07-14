import { appTitle } from 'src/utils/format-string';

import { WalletsView } from 'src/sections/wallet/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Wallets'),
};

export default function WalletsPage() {
  return <WalletsView />;
}
