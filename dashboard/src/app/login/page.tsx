import { appTitle } from 'src/utils/format-string';

import { CONFIG } from 'src/global-config';

import { JwtSignInView } from 'src/auth/view/jwt';
import { Auth0SignInView } from 'src/auth/view/auth0';
import { SupabaseSignInView } from 'src/auth/view/supabase';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Sign In') };

export default function Page() {
  switch (CONFIG.auth.method) {
    case 'auth0':
      return <Auth0SignInView />;
    case 'supabase':
      return <SupabaseSignInView />;
    default:
      return <JwtSignInView />;
  }
}
