import { paths } from 'src/routes/paths';

import { CONFIG } from 'src/config-global';

import { Iconify } from 'src/components/iconify';

import type { AccountDrawerProps } from './components/account-drawer';

// ----------------------------------------------------------------------

export const accountNavData: AccountDrawerProps['data'] = [
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
    href: 'https://docs.numeraire.tech',
    icon: <Iconify icon="solar:book-bold-duotone" />,
    target: '_blank',
  },
  {
    label: 'API Reference',
    href: `${CONFIG.site.serverUrl}/docs`,
    icon: <Iconify icon="solar:code-bold-duotone" />,
    target: '_blank',
  },
  {
    label: 'Support',
    href: 'https://numeraire.tech/contact',
    icon: <Iconify icon="solar:help-bold-duotone" />,
    target: '_blank',
  },
];
