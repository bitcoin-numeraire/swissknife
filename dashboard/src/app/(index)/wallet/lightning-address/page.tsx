import { redirect } from 'next/navigation';

import { appTitle } from 'src/utils/format-string';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Lightning Address'),
};

export default function LnAddressDetailsPage() {
  redirect('/identity?tab=lightning');
}
