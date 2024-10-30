import { appTitle } from 'src/utils/format-string';

import { CONFIG } from 'src/config-global';

import { NotFoundView } from 'src/sections/error';

// ----------------------------------------------------------------------

export const metadata = { title: appTitle(`404 page not found! | Error - ${CONFIG.site.name}`) };

export default function NotFoundPage() {
  return <NotFoundView />;
}
