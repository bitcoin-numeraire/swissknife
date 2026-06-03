import { appTitle } from 'src/utils/format-string';

import { NodeView } from 'src/sections/node/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Node Management'),
};

export default function OverviewBankingPage() {
  return <NodeView />;
}
