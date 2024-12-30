import { Alert } from '@mui/material';

import { appTitle } from 'src/utils/format-string';

import { DashboardContent } from 'src/layouts/dashboard';

import { NodeView, BreezNodeView } from 'src/sections/node/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Node management'),
};

export default function OverviewBankingPage() {
  switch (process.env.LN_PROVIDER) {
    case 'breez':
      return <BreezNodeView />;
    case 'cln':
      return <NodeView />;
    case 'lnd':
      return <NodeView />;
    default:
      return (
        <DashboardContent>
          <Alert severity="error">Page not available for Non Breez Lightning Provider</Alert>
        </DashboardContent>
      );
  }
}
