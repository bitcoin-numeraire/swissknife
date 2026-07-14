import type { WalletOverview } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import { Protocol, BtcNetwork } from 'src/lib/swissknife';

import { sortWallets, walletReadiness, walletMatchesSearch } from './wallets-view';

function overview(overrides: Partial<WalletOverview> = {}) {
  return {
    id: 'wallet-mainnet-btc',
    account_id: 'account-treasury',
    asset_id: 'asset-mainnet-btc',
    asset: {
      id: 'asset-mainnet-btc',
      code: 'BTC',
      name: 'Bitcoin',
      protocol: Protocol.BITCOIN,
      network: BtcNetwork.BITCOIN,
      asset_ref: 'native',
      display_ticker: 'BTC',
      decimals: 11,
      created_at: new Date('2026-01-01T00:00:00Z'),
    },
    label: 'Treasury',
    balance: {
      received_msat: 0,
      sent_msat: 0,
      fees_paid_msat: 0,
      reserved_msat: 0,
      available_msat: 0,
    },
    n_payments: 2,
    n_invoices: 3,
    n_contacts: 1,
    created_at: new Date('2026-01-01T00:00:00Z'),
    ...overrides,
  } as WalletOverview;
}

describe('wallet directory', () => {
  it.each(['treasury', 'WALLET-MAINNET', 'account-treasury', 'btc', 'bitcoin'])(
    'matches wallet, account, and asset data for %s',
    (query) => {
      expect(walletMatchesSearch(overview(), query)).toBe(true);
    }
  );

  it('derives Lightning Address readiness', () => {
    expect(walletReadiness(overview())).toBe('missing');
    expect(
      walletReadiness(
        overview({
          ln_address: { active: false, username: 'treasury' } as WalletOverview['ln_address'],
        })
      )
    ).toBe('paused');
    expect(
      walletReadiness(
        overview({
          ln_address: { active: true, username: 'treasury' } as WalletOverview['ln_address'],
        })
      )
    ).toBe('ready');
  });

  it('sorts activity without comparing balances across assets or networks', () => {
    const quieter = overview({ id: 'quiet', n_payments: 0, n_invoices: 0, n_contacts: 0 });
    const busier = overview({ id: 'busy', n_payments: 4, n_invoices: 5, n_contacts: 2 });

    expect(sortWallets([quieter, busier], 'activity').map((wallet) => wallet.id)).toEqual([
      'busy',
      'quiet',
    ]);
  });
});
