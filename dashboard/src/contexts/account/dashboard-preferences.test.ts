import type { SettingsState } from 'src/components/settings';

import { it, expect, describe } from 'vitest';

import { dashboardPreferencePatch, uiSettingsFromDashboardSettings } from './dashboard-preferences';

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

  it('sends only the changed fields, including a switch back to visible balances', () => {
    expect(dashboardPreferencePatch({ hideBalances: false })).toEqual({ hide_balances: false });
    expect(dashboardPreferencePatch({ mode: 'dark', defaultAddressType: 'p2tr' })).toEqual({
      theme: 'dark',
      default_address_type: 'p2tr',
    });
  });
});
