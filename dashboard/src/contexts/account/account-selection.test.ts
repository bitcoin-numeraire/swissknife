import type { Wallet } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import {
  selectInitialWalletId,
  ACTIVE_WALLET_SETTING,
  settingsWithActiveWallet,
  DASHBOARD_SETTINGS_SCHEMA_VERSION,
} from './account-selection';

const wallet = (id: string) => ({ id }) as Wallet;

describe('selectInitialWalletId', () => {
  it('keeps the current wallet while it remains available', () => {
    expect(
      selectInitialWalletId([wallet('wallet-1'), wallet('wallet-2')], 'wallet-2', {
        [ACTIVE_WALLET_SETTING]: 'wallet-1',
      })
    ).toBe('wallet-2');
  });

  it('uses the persisted wallet before falling back to the first wallet', () => {
    expect(
      selectInitialWalletId([wallet('wallet-1'), wallet('wallet-2')], undefined, {
        [ACTIVE_WALLET_SETTING]: 'wallet-2',
      })
    ).toBe('wallet-2');
  });

  it('handles an account without wallets', () => {
    expect(selectInitialWalletId([], undefined, {})).toBeUndefined();
  });
});

describe('settingsWithActiveWallet', () => {
  it('preserves unrelated dashboard settings', () => {
    expect(settingsWithActiveWallet({ theme: 'dark' }, 'wallet-2')).toEqual({
      theme: 'dark',
      schema_version: DASHBOARD_SETTINGS_SCHEMA_VERSION,
      [ACTIVE_WALLET_SETTING]: 'wallet-2',
    });
  });
});
