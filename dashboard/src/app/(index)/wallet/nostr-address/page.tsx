import { redirect } from 'next/navigation';

import { appTitle } from 'src/utils/format-string';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Nostr Address'),
};

export default function LnAddressDetailsPage() {
  redirect('/identity?tab=nostr');
}
