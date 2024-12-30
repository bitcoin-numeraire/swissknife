import { CONFIG } from 'src/global-config';

import type { WorkspacesPopoverProps } from './components/workspaces-popover';

// ----------------------------------------------------------------------

export const _workspaces: WorkspacesPopoverProps['data'] = [
  {
    id: 'main',
    name: 'Main',
    logo: `${CONFIG.assetsDir}/assets/icons/bitcoin/ic-bitcoin.svg`,
    plan: 'BTC',
  },
];
