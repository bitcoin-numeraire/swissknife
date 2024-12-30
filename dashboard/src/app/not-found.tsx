import type { Metadata } from 'next';

import { appTitle } from 'src/utils/format-string';

import { NotFoundView } from 'src/sections/error';

// ----------------------------------------------------------------------

export const metadata: Metadata = { title: appTitle(`404 page not found! | Error`) };

export default function Page() {
  return <NotFoundView />;
}
