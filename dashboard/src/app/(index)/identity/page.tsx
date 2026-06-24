import { appTitle } from 'src/utils/format-string';

import { IdentityView } from 'src/sections/identity/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Identity'),
};

export default function IdentityPage() {
  return <IdentityView />;
}
