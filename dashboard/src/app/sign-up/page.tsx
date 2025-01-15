import { appTitle } from 'src/utils/format-string';

import { JwtSignUpView } from 'src/auth/view/jwt';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Sign Up') };

export default function Page() {
  return <JwtSignUpView />;
}
