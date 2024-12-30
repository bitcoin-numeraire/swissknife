import { appTitle } from 'src/utils/format-string';

import { CONFIG } from 'src/global-config';

import { JwtSignUpView } from 'src/auth/view/jwt';
import { SupabaseSignUpView } from 'src/auth/view/supabase';

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
