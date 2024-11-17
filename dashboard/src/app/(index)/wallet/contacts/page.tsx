import { appTitle } from 'src/utils/format-string';

import { ContactListView } from 'src/sections/contact/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Contacts'),
};

export default function LnAddressDetailsPage() {
  return <ContactListView />;
}
