import type { NavGroupProps } from 'src/components/nav-section';

import { paths } from 'src/routes/paths';

import { CONFIG } from 'src/config-global';
import { Permission } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';
import { SvgColor } from 'src/components/svg-color';

// ----------------------------------------------------------------------

const icon = (name: string) => <SvgColor src={`${CONFIG.site.basePath}/assets/icons/navbar/${name}.svg`} />;
const iconify = (name: string) => <Iconify icon={name} sx={{ width: 1, height: 1 }} />;

const ICONS = {
  user: icon('ic-user'),
  lock: icon('ic-lock'),
  label: icon('ic-label'),
  disabled: icon('ic-disabled'),
  external: icon('ic-external'),
  menuItem: icon('ic-menu-item'),
  dashboard: icon('ic-dashboard'),
  parameter: icon('ic-parameter'),
  wallet: iconify('solar:wallet-bold-duotone'),
  node: iconify('solar:server-minimalistic-bold-duotone'),
  invoice: iconify('eva:diagonal-arrow-left-down-fill'),
  payment: iconify('eva:diagonal-arrow-right-up-fill'),
  lightning: iconify('solar:bolt-bold-duotone'),
  nostr: <SvgColor src={`${CONFIG.site.basePath}/assets/icons/navbar/ic-nostr.svg`} />,
  contacts: iconify('solar:users-group-rounded-bold-duotone'),
  apiKeys: iconify('solar:code-bold-duotone'),
};

// ----------------------------------------------------------------------

export const navData: Array<NavGroupProps> = [
  /**
   * User Wallet
   */
  {
    subheader: 'wallet',
    items: [
      {
        title: 'overview',
        path: paths.wallet.root,
        icon: ICONS.wallet,
      },
      {
        title: 'payments',
        path: paths.wallet.payments,
        icon: ICONS.payment,
      },
      {
        title: 'invoices',
        path: paths.wallet.invoices,
        icon: ICONS.invoice,
      },
      {
        title: 'lightning_address',
        path: paths.wallet.lightningAddress,
        icon: ICONS.lightning,
      },
      {
        title: 'nostr_address',
        path: paths.wallet.nostrAddress,
        icon: ICONS.nostr,
      },
      {
        title: 'contacts',
        path: paths.wallet.contacts,
        icon: ICONS.contacts,
      },
    ],
  },
  /**
   * Administration
   */
  {
    subheader: 'administration',
    items: [
      {
        title: 'node',
        path: paths.admin.node,
        icon: ICONS.node,
        permissions: [Permission.READ_TRANSACTION, Permission.READ_LN_NODE, Permission.READ_LN_ADDRESS],
      },
      {
        title: 'wallets',
        path: paths.admin.wallets,
        icon: ICONS.wallet,
        permissions: [Permission.READ_WALLET],
      },
      {
        title: 'payments',
        path: paths.admin.payments,
        icon: ICONS.payment,
        permissions: [Permission.READ_TRANSACTION],
      },
      {
        title: 'invoices',
        path: paths.admin.invoices,
        icon: ICONS.invoice,
        permissions: [Permission.READ_TRANSACTION],
      },
      {
        title: 'lightning_addresses',
        path: paths.admin.lnAddresses,
        icon: ICONS.lightning,
        permissions: [Permission.READ_LN_ADDRESS],
      },
      {
        title: 'api_keys',
        path: paths.admin.apiKeys,
        icon: ICONS.apiKeys,
        permissions: [Permission.READ_API_KEY],
      },
    ],
  },
];
