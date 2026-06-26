import { redirect } from 'next/navigation';

import { paths } from 'src/routes/paths';

import { appTitle } from 'src/utils/format-string';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('API Keys Management'),
};

export default function AdminApiKeyListPage() {
  redirect(paths.build.apiKeys);
}
