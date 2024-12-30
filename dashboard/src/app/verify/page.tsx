import { appTitle } from 'src/utils/format-string';

import { SupabaseVerifyView } from 'src/auth/view/supabase';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle('Verify | Supabase') };

export default function Page() {
  return <SupabaseVerifyView />;
}
