import { appTitle } from 'src/utils/format-string';

import { SupabaseResetPasswordView } from 'src/auth/view/supabase';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Reset password | Supabase') };

export default function Page() {
  return <SupabaseResetPasswordView />;
}
