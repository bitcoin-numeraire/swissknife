import { appTitle } from 'src/utils/format-string';

import { LnAddressListView } from 'src/sections/ln-address/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Lightning Addresses Management'),
};

export default function LnAddressListPage() {
  return <LnAddressListView />;
}
