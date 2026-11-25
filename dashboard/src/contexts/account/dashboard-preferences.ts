import type { SettingsState } from 'src/components/settings';

import { dashboardSettings } from './account-selection';

export type DashboardPreferenceUpdate = Partial<
  Pick<SettingsState, 'mode' | 'currency' | 'displayUnit' | 'hideBalances' | 'defaultAddressType'>
>;

function isOneOf<T extends string>(value: unknown, values: readonly T[]): value is T {
  return typeof value === 'string' && values.includes(value as T);
}

export function uiSettingsFromDashboardSettings(
  storedSettings: unknown,
  defaults: SettingsState
): DashboardPreferenceUpdate {
  const stored = dashboardSettings(storedSettings);

  return {
    mode: isOneOf(stored.theme, ['light', 'dark', 'system']) ? stored.theme : defaults.mode,
    currency: isOneOf(stored.currency, ['USD', 'EUR', 'CHF']) ? stored.currency : defaults.currency,
    displayUnit: isOneOf(stored.display_unit, ['bip177', 'sats'])
      ? stored.display_unit
      : defaults.displayUnit,
    hideBalances:
      typeof stored.hide_balances === 'boolean' ? stored.hide_balances : defaults.hideBalances,
    defaultAddressType: isOneOf(stored.default_address_type, ['p2tr', 'p2wpkh'])
      ? stored.default_address_type
      : defaults.defaultAddressType,
  };
}

export function dashboardPreferencePatch(
  update: DashboardPreferenceUpdate
): Record<string, unknown> {
  const patch: Record<string, unknown> = {};
  if (update.mode !== undefined) patch.theme = update.mode;
  if (update.currency !== undefined) patch.currency = update.currency;
  if (update.displayUnit !== undefined) patch.display_unit = update.displayUnit;
  if (update.hideBalances !== undefined) patch.hide_balances = update.hideBalances;
  if (update.defaultAddressType !== undefined)
    patch.default_address_type = update.defaultAddressType;
  return patch;
}
