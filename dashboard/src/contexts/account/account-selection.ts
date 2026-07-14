import type { Wallet } from 'src/lib/swissknife';

export const ACTIVE_WALLET_SETTING = 'active_wallet_id';
export const DASHBOARD_SETTINGS_SCHEMA_VERSION = 1;

export function dashboardSettings(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return value as Record<string, unknown>;
}

export function selectInitialWalletId(
  wallets: Wallet[],
  currentWalletId: string | undefined,
  storedSettings: unknown
) {
  if (currentWalletId && wallets.some((wallet) => wallet.id === currentWalletId)) {
    return currentWalletId;
  }

  const preferredWalletId = dashboardSettings(storedSettings)[ACTIVE_WALLET_SETTING];
  if (
    typeof preferredWalletId === 'string' &&
    wallets.some((wallet) => wallet.id === preferredWalletId)
  ) {
    return preferredWalletId;
  }

  return wallets[0]?.id;
}

export function settingsWithActiveWallet(storedSettings: unknown, walletId: string) {
  return {
    ...dashboardSettings(storedSettings),
    schema_version: DASHBOARD_SETTINGS_SCHEMA_VERSION,
    [ACTIVE_WALLET_SETTING]: walletId,
  };
}
