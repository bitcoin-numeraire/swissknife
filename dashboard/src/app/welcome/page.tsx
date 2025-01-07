import { appTitle } from 'src/utils/format-string';

import { WelcomeView } from 'src/sections/onboarding/view';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Welcome') };

export default function Page() {
  <WelcomeView />;
}
