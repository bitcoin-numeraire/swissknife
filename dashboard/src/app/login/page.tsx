import { appTitle } from 'src/utils/format-string';

import { CONFIG } from 'src/config-global';

import { JwtSignInView } from 'src/sections/auth/jwt';
import { Auth0SignInView } from 'src/sections/auth/auth0';
import { SupabaseSignInView } from 'src/sections/auth/supabase';

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
