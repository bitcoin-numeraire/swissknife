import type { Wallet, Account } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import { AuthProvider } from 'src/lib/swissknife';

import { accountMatchesSearch } from './accounts-view';

const account = {
  id: 'account-1',
  display_name: 'Treasury',
  created_at: new Date(),
  identity: {
    id: 'identity-1',
    provider: AuthProvider.OAUTH2,
    subject: 'auth0|operator',
  },
} as Account;

const wallet = {
  id: 'wallet-mainnet-btc',
  account_id: account.id,
} as Wallet;

describe('account directory search', () => {
  it.each(['treasury', 'ACCOUNT-1', 'auth0', 'operator', 'wallet-mainnet'])(
    'matches account and related wallet data for %s',
    (query) => {
      expect(accountMatchesSearch(account, [wallet], query)).toBe(true);
    }
  );

  it('does not match unrelated data', () => {
    expect(accountMatchesSearch(account, [wallet], 'testnet')).toBe(false);
  });
});
