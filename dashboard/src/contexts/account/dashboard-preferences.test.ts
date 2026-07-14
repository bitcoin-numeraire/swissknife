import type { SettingsState } from 'src/components/settings';

import { it, expect, describe } from 'vitest';

import {
  settingsWithUiPreferences,
  uiSettingsFromDashboardSettings,
  DASHBOARD_SETTINGS_SCHEMA_VERSION,
} from './dashboard-preferences';

const defaults = {
  mode: 'system',
  currency: 'USD',
  displayUnit: 'bip177',
  hideBalances: false,
  defaultAddressType: 'p2tr',
} as SettingsState;

describe('dashboard preferences', () => {
  it('hydrates supported settings and falls back when stored values are invalid', () => {
    expect(
      uiSettingsFromDashboardSettings(
        {
          theme: 'dark',
          currency: 'BTC',
          display_unit: 'sats',
          hide_balances: true,
          default_address_type: 'legacy',
        },
        defaults
      )
    ).toEqual({
      mode: 'dark',
      currency: 'USD',
      displayUnit: 'sats',
      hideBalances: true,
      defaultAddressType: 'p2tr',
    });
  });

  it('preserves active-wallet and unknown settings when updating UI preferences', () => {
    expect(
      settingsWithUiPreferences(
        { active_wallet_id: 'wallet-2', future_setting: true },
        {
          ...defaults,
          mode: 'light',
          currency: 'CHF',
          displayUnit: 'sats',
          hideBalances: true,
          defaultAddressType: 'p2wpkh',
        }
      )
    ).toEqual({
      schema_version: DASHBOARD_SETTINGS_SCHEMA_VERSION,
      active_wallet_id: 'wallet-2',
      future_setting: true,
      theme: 'light',
      currency: 'CHF',
      display_unit: 'sats',
      hide_balances: true,
      default_address_type: 'p2wpkh',
    });
  });
});
