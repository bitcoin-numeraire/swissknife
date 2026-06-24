import type { DeploymentMode } from 'src/global-config';
import type { NavGroupProps, NavItemDataProps } from 'src/components/nav-section';

import { paths } from 'src/routes/paths';

import { CONFIG } from 'src/global-config';
import { Permission } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';
import { SvgColor } from 'src/components/svg-color';

import { hasAllPermissions } from 'src/auth/permissions';

// ----------------------------------------------------------------------

const icon = (name: string) => (
  <SvgColor src={`${CONFIG.assetsDir}/assets/icons/navbar/${name}.svg`} />
);
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
  nostr: <SvgColor src={`${CONFIG.assetsDir}/assets/icons/navbar/ic-nostr.svg`} />,
  contacts: iconify('solar:users-group-rounded-bold-duotone'),
  apiKeys: iconify('solar:code-bold-duotone'),
  activity: iconify('solar:bill-list-bold-duotone'),
  identity: iconify('solar:fingerprint-bold-duotone'),
  accounts: iconify('solar:users-group-two-rounded-bold-duotone'),
  observe: iconify('solar:pulse-2-bold-duotone'),
  policy: iconify('solar:shield-keyhole-bold-duotone'),
  webhooks: iconify('solar:programming-bold-duotone'),
};

// ----------------------------------------------------------------------

// Swissknife-specific: nav items carry the API permissions required to view
// them (consumed by the searchbar permission filtering).
type NavItemWithPermissions = NavItemDataProps & {
  permissions?: string[];
  modes?: DeploymentMode[] | 'all';
  flag?: 'agents' | 'policy' | 'webhooks' | 'l402' | 'node-health';
  children?: NavItemWithPermissions[];
};

type NavGroupWithPermissions = Omit<NavGroupProps, 'items'> & {
  items: NavItemWithPermissions[];
};

export const navData: Array<NavGroupWithPermissions> = [
  /**
   * User Wallet
   */
  {
    subheader: 'money',
    items: [
      {
        title: 'overview',
        path: paths.overview,
        icon: ICONS.wallet,
      },
      {
        title: 'activity',
        path: paths.activity,
        icon: ICONS.activity,
        permissions: [Permission.READ_TRANSACTION],
      },
    ],
  },
  {
    subheader: 'identity',
    items: [
      {
        title: 'identity_hub',
        path: paths.identity,
        icon: ICONS.identity,
        permissions: [Permission.READ_LN_ADDRESS],
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
    subheader: 'accounts',
    items: [
      {
        title: 'accounts_directory',
        path: paths.accounts,
        icon: ICONS.accounts,
        permissions: [Permission.READ_WALLET],
        modes: ['server', 'self-hosted', 'merchant'],
      },
      {
        title: 'roles_access',
        path: paths.admin.wallets,
        icon: ICONS.policy,
        permissions: [Permission.READ_WALLET],
        modes: ['server', 'merchant'],
      },
    ],
  },
  {
    subheader: 'observe',
    items: [
      {
        title: 'node_health',
        path: paths.nodeHealth,
        icon: ICONS.observe,
        permissions: [Permission.READ_LN_NODE],
        modes: ['server', 'self-hosted', 'desktop'],
      },
      {
        title: 'volume_reconciliation',
        path: paths.admin.node,
        icon: ICONS.node,
        permissions: [Permission.READ_TRANSACTION],
        modes: ['server', 'merchant'],
      },
    ],
  },
  {
    subheader: 'build',
    items: [
      {
        title: 'api_keys',
        path: paths.admin.apiKeys,
        icon: ICONS.apiKeys,
        permissions: [Permission.READ_API_KEY],
        modes: ['server', 'desktop', 'agent'],
      },
      {
        title: 'agents',
        path: paths.admin.apiKeys,
        icon: ICONS.accounts,
        permissions: [Permission.READ_API_KEY, Permission.READ_WALLET],
        modes: ['server', 'agent'],
        flag: 'agents',
      },
      {
        title: 'webhooks',
        path: paths.admin.apiKeys,
        icon: ICONS.webhooks,
        permissions: [Permission.READ_API_KEY],
        modes: ['server', 'agent'],
        flag: 'webhooks',
      },
    ],
  },
];

const dashboardFlags = new Set(
  (process.env.NEXT_PUBLIC_DASHBOARD_FLAGS ?? '')
    .split(',')
    .map((flag) => flag.trim())
    .filter(Boolean)
);

function canRenderItem(
  item: NavItemWithPermissions,
  userPermissions: string[],
  mode: DeploymentMode
) {
  const modes = item.modes ?? 'all';
  const hasMode = modes === 'all' || modes.includes(mode);
  const hasFlag = !item.flag || dashboardFlags.has(item.flag);
  const hasPermissions = hasAllPermissions(item.permissions, userPermissions);

  return hasMode && hasFlag && hasPermissions;
}

function filterItems(
  items: NavItemWithPermissions[],
  userPermissions: string[],
  mode: DeploymentMode
): NavItemWithPermissions[] {
  return items
    .filter((item) => canRenderItem(item, userPermissions, mode))
    .map((item) => ({
      ...item,
      children: item.children ? filterItems(item.children, userPermissions, mode) : undefined,
    }));
}

export function filterDashboardNavData(
  data: Array<NavGroupWithPermissions>,
  userPermissions: string[] = [],
  mode: DeploymentMode = CONFIG.deploymentMode
): Array<NavGroupWithPermissions> {
  return data
    .map((group) => ({
      ...group,
      items: filterItems(group.items, userPermissions, mode),
    }))
    .filter((group) => group.items.length > 0);
}
