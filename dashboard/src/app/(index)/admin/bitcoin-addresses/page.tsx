import { appTitle } from 'src/utils/format-string';

import { BtcAddressesView } from 'src/sections/btc-address/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Bitcoin Addresses Management'),
};

export default function BtcAddressListPage() {
  return <BtcAddressesView />;
}
