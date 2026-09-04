import { appTitle } from 'src/utils/format-string';

import { CONFIG } from 'src/global-config';

import { SupabaseResetPasswordView } from 'src/auth/view/supabase';
import { JwtResetPasswordView } from 'src/auth/view/jwt/jwt-reset-password-view';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Reset password') };

export default function Page() {
  return CONFIG.auth.method === 'jwt' ? <JwtResetPasswordView /> : <SupabaseResetPasswordView />;
}
