import { appTitle } from 'src/utils/format-string';

import { NostrDetailsView } from 'src/sections/nostr/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Nostr Address'),
};

export default function LnAddressDetailsPage() {
  return <NostrDetailsView />;
}
