import { appTitle } from 'src/utils/format-string';

import { LnAddressesView } from 'src/sections/ln-address/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('LN Addresses'),
};

export default function LnAddressListPage() {
  return <LnAddressesView />;
}
