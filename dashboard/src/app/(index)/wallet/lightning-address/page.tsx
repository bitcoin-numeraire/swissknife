import { appTitle } from 'src/utils/format-string';

import { LnAddressDetailsView } from 'src/sections/ln-address/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Lightning Address'),
};

export default function LnAddressDetailsPage() {
  return <LnAddressDetailsView />;
}
