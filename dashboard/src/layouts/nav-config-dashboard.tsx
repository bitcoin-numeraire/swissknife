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
  invoice: iconify('eva:diagonal-arrow-left-down-fill'),
  payment: iconify('eva:diagonal-arrow-right-up-fill'),
  lightning: iconify('solar:bolt-bold-duotone'),
  bitcoinAddress: iconify('solar:link-round-angle-bold-duotone'),
  nostr: <SvgColor src={`${CONFIG.assetsDir}/assets/icons/navbar/ic-nostr.svg`} />,
  contacts: iconify('solar:users-group-rounded-bold-duotone'),
  apiKeys: iconify('solar:code-bold-duotone'),
  activity: iconify('solar:bill-list-bold-duotone'),
  identity: iconify('solar:user-rounded-bold-duotone'),
  accounts: iconify('solar:users-group-two-rounded-bold-duotone'),
  policy: iconify('solar:shield-keyhole-bold-duotone'),
  webhooks: iconify('solar:programming-bold-duotone'),
  adminWallets: iconify('solar:safe-square-bold-duotone'),
};

// ----------------------------------------------------------------------

// Swissknife-specific: nav items carry the API permissions required to view
// them (consumed by the searchbar permission filtering).
export type DashboardNavFlag = 'agents' | 'policy' | 'webhooks' | 'l402';

export type NavItemWithPermissions = NavItemDataProps & {
  permissions?: string[];
  modes?: DeploymentMode[] | 'all';
  flag?: DashboardNavFlag;
  children?: NavItemWithPermissions[];
};

export type NavGroupWithPermissions = Omit<NavGroupProps, 'items'> & {
  items: NavItemWithPermissions[];
};

export type DashboardModuleGateReason =
  | {
      type: 'permissions';
      missing: string[];
    }
  | {
      type: 'mode';
      activeMode: DeploymentMode;
      allowedModes: DeploymentMode[];
    }
  | {
      type: 'flag';
      flag: DashboardNavFlag;
    };

export type DashboardModuleDiagnostic = {
  group: string;
  title: string;
  path?: string;
  permissions: string[];
  modes: DeploymentMode[] | 'all';
  flag?: DashboardNavFlag;
  visible: boolean;
  reasons: DashboardModuleGateReason[];
  depth: number;
};

export const navData: Array<NavGroupWithPermissions> = [
  /**
   * Account wallet
   */
  {
    subheader: 'wallet',
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
        title: 'admin_transactions',
        path: paths.admin.transactions,
        icon: ICONS.activity,
        permissions: [Permission.READ_TRANSACTION],
        modes: ['server', 'self-hosted', 'merchant'],
      },
      {
        title: 'admin_lightning_addresses',
        path: paths.admin.lnAddresses,
        icon: ICONS.lightning,
        permissions: [Permission.READ_LN_ADDRESS],
        modes: ['server', 'self-hosted', 'merchant'],
      },
      {
        title: 'admin_bitcoin_addresses',
        path: paths.admin.btcAddresses,
        icon: ICONS.bitcoinAddress,
        permissions: [Permission.READ_BTC_ADDRESS],
        modes: ['server', 'self-hosted', 'merchant'],
      },
    ],
  },
  {
    subheader: 'build',
    items: [
      {
        title: 'api_keys',
        path: paths.build.apiKeys,
        icon: ICONS.apiKeys,
        modes: ['server', 'desktop', 'agent'],
      },
      {
        title: 'agents',
        path: paths.build.apiKeys,
        icon: ICONS.accounts,
        permissions: [Permission.READ_API_KEY, Permission.READ_WALLET],
        modes: ['server', 'agent'],
        flag: 'agents',
      },
      {
        title: 'webhooks',
        path: paths.build.apiKeys,
        icon: ICONS.webhooks,
        permissions: [Permission.READ_API_KEY],
        modes: ['server', 'agent'],
        flag: 'webhooks',
      },
    ],
  },
];

export function getDashboardFlags(flags = process.env.NEXT_PUBLIC_DASHBOARD_FLAGS ?? '') {
  return new Set(
    flags
      .split(',')
      .map((flag) => flag.trim())
      .filter(Boolean)
  );
}

const dashboardFlags = getDashboardFlags();

function getGateReasons(
  item: NavItemWithPermissions,
  userPermissions: string[],
  mode: DeploymentMode,
  enabledFlags: Set<string>
): DashboardModuleGateReason[] {
  const modes = item.modes ?? 'all';
  const hasPermissions = hasAllPermissions(item.permissions, userPermissions);
  const missingPermissions = hasPermissions
    ? []
    : (item.permissions ?? []).filter((permission) => !userPermissions.includes(permission));

  return [
    ...(missingPermissions.length > 0
      ? [{ type: 'permissions' as const, missing: missingPermissions }]
      : []),
    ...(modes !== 'all' && !modes.includes(mode)
      ? [{ type: 'mode' as const, activeMode: mode, allowedModes: modes }]
      : []),
    ...(item.flag && !enabledFlags.has(item.flag)
      ? [{ type: 'flag' as const, flag: item.flag }]
      : []),
  ];
}

function getModuleDiagnostics(
  group: NavGroupWithPermissions,
  userPermissions: string[],
  mode: DeploymentMode,
  enabledFlags: Set<string>,
  items: NavItemWithPermissions[] = group.items,
  depth = 0
): DashboardModuleDiagnostic[] {
  return items.flatMap((item) => {
    const reasons = getGateReasons(item, userPermissions, mode, enabledFlags);
    const diagnostic: DashboardModuleDiagnostic = {
      group: group.subheader ?? 'ungrouped',
      title: item.title,
      path: item.path,
      permissions: item.permissions ?? [],
      modes: item.modes ?? 'all',
      flag: item.flag,
      visible: reasons.length === 0,
      reasons,
      depth,
    };

    return [
      diagnostic,
      ...(item.children
        ? getModuleDiagnostics(group, userPermissions, mode, enabledFlags, item.children, depth + 1)
        : []),
    ];
  });
}

export function getDashboardModuleDiagnostics(
  data: Array<NavGroupWithPermissions>,
  userPermissions: string[] = [],
  mode: DeploymentMode = CONFIG.deploymentMode,
  enabledFlags: Set<string> = dashboardFlags
): DashboardModuleDiagnostic[] {
  return data.flatMap((group) => getModuleDiagnostics(group, userPermissions, mode, enabledFlags));
}

function filterItems(
  items: NavItemWithPermissions[],
  userPermissions: string[],
  mode: DeploymentMode
): NavItemWithPermissions[] {
  return items
    .filter((item) => getGateReasons(item, userPermissions, mode, dashboardFlags).length === 0)
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
