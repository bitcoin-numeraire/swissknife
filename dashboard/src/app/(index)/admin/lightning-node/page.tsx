import { appTitle } from 'src/utils/format-string';

import { NodeView, BreezNodeView } from 'src/sections/node/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Node Management'),
};

export default function OverviewBankingPage() {
  switch (process.env.LN_PROVIDER) {
    case 'breez':
      return <BreezNodeView />;
    default:
      return <NodeView />;
  }
}
