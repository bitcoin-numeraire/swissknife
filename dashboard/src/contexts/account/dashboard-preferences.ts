import type { SettingsState } from 'src/components/settings';

import { dashboardSettings, DASHBOARD_SETTINGS_SCHEMA_VERSION } from './account-selection';

export { DASHBOARD_SETTINGS_SCHEMA_VERSION } from './account-selection';

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

export function settingsWithUiPreferences(storedSettings: unknown, state: SettingsState) {
  return {
    ...dashboardSettings(storedSettings),
    schema_version: DASHBOARD_SETTINGS_SCHEMA_VERSION,
    theme: state.mode,
    currency: state.currency,
    display_unit: state.displayUnit,
    hide_balances: state.hideBalances,
    default_address_type: state.defaultAddressType,
  };
}
