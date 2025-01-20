import { appTitle } from 'src/utils/format-string';

import { LnAddressesView } from 'src/sections/ln-address/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Lightning Addresses Management'),
};

export default function LnAddressListPage() {
  return <LnAddressesView />;
}
