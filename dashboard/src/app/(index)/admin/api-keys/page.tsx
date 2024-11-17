import { appTitle } from 'src/utils/format-string';

import { ApiKeyListView } from 'src/sections/api-key/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('API Keys Management'),
};

export default function AdminApiKeyListPage() {
  return <ApiKeyListView />;
}
