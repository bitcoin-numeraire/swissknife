import { appTitle } from 'src/utils/format-string';

import { CONFIG } from 'src/config-global';

import { JwtSignUpView } from 'src/sections/auth/jwt';
import { SupabaseSignUpView } from 'src/sections/auth/supabase';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Sign Up') };

export default function Page() {
  switch (CONFIG.auth.method) {
    case 'supabase':
      return <SupabaseSignUpView />;
    default:
      return <JwtSignUpView />;
  }
}
