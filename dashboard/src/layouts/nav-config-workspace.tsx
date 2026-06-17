import type { WorkspacesPopoverProps } from './components/workspaces-popover';

import { CONFIG } from 'src/global-config';

// ----------------------------------------------------------------------

export const _workspaces: WorkspacesPopoverProps['data'] = [
  {
    id: 'main',
    name: 'Main',
    logo: `${CONFIG.assetsDir}/assets/icons/bitcoin/ic-bitcoin.svg`,
    plan: 'BTC',
  },
];
