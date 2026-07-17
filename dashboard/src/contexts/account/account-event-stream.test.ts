import { it, expect, describe } from 'vitest';

import { endpointKeys } from 'src/actions/keys';

import { isWalletEventCacheKey } from './account-event-stream';

describe('wallet event cache invalidation', () => {
  const walletId = 'wallet-1';

  it('refreshes the wallet balance and transaction resources', () => {
    expect(isWalletEventCacheKey(endpointKeys.accountWallet.balance(walletId), walletId)).toBe(
      true
    );
    expect(
      isWalletEventCacheKey(endpointKeys.accountWallet.payments.list(walletId, 25, 0), walletId)
    ).toBe(true);
    expect(
      isWalletEventCacheKey(
        endpointKeys.accountWallet.invoices.get(walletId, 'invoice-1'),
        walletId
      )
    ).toBe(true);
  });

  it('does not refresh another wallet cache', () => {
    expect(isWalletEventCacheKey(endpointKeys.accountWallet.get('wallet-2'), walletId)).toBe(false);
  });

  it('refreshes shared dashboard aggregates', () => {
    expect(isWalletEventCacheKey(endpointKeys.account.get, walletId)).toBe(true);
    expect(isWalletEventCacheKey(endpointKeys.wallets.listOverviews, walletId)).toBe(true);
  });
});
