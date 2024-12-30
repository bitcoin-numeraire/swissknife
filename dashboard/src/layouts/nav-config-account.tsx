import { paths } from 'src/routes/paths';

import { CONFIG } from 'src/global-config';

import { Iconify } from 'src/components/iconify';

import type { AccountDrawerProps } from './components/account-drawer';

// ----------------------------------------------------------------------

export const navData: AccountDrawerProps['data'] = [
  {
    label: 'Home',
    href: paths.wallet.root,
    icon: <Iconify icon="solar:home-angle-bold-duotone" />,
  },
  {
    label: 'Settings',
    href: paths.settings.root,
    icon: <Iconify icon="solar:settings-bold-duotone" />,
  },
  {
    label: 'Documentation',
    href: paths.external.numeraire.docs,
    icon: <Iconify icon="solar:book-bold-duotone" />,
    target: '_blank',
  },
  {
    label: 'API Reference',
    href: `${CONFIG.serverUrl}/docs`,
    icon: <Iconify icon="solar:code-bold-duotone" />,
    target: '_blank',
  },
  {
    label: 'Support',
    href: paths.external.numeraire.contact,
    icon: <Iconify icon="solar:help-bold-duotone" />,
    target: '_blank',
  },
];
