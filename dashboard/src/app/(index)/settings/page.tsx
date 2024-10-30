import { appTitle } from 'src/utils/format-string';

import { SettingsView } from 'src/sections/settings/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Settings'),
};

export default function Page() {
  return <SettingsView />;
}
