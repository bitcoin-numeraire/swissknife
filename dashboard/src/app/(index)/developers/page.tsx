import { Suspense } from 'react';

import { appTitle } from 'src/utils/format-string';

import { LoadingScreen } from 'src/components/loading-screen';

import { DevelopersView } from 'src/sections/developers/view';

export const metadata = { title: appTitle('Developers') };

export default function Page() {
  return (
    <Suspense fallback={<LoadingScreen />}>
      <DevelopersView />
    </Suspense>
  );
}
