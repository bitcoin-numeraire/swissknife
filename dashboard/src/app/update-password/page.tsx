import { appTitle } from 'src/utils/format-string';

import { SupabaseUpdatePasswordView } from 'src/auth/view/supabase';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Update password | Supabase') };

export default function Page() {
  return <SupabaseUpdatePasswordView />;
}
