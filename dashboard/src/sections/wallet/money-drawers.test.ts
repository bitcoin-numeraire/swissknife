import { it, expect, describe } from 'vitest';

import { getReceiveAddressListState } from './receive-address-list';

describe('getReceiveAddressListState', () => {
  it('uses the account-wallet address list for the regular receive drawer', () => {
    expect(
      getReceiveAddressListState({
        open: true,
        isAdmin: false,
        selectedNeedsAddress: true,
        addressWalletId: 'wallet-1',
      })
    ).toEqual({
      adminQuery: undefined,
      adminEnabled: false,
      walletEnabled: true,
    });
  });

  it('uses the admin address list when receiving for a selected wallet', () => {
    expect(
      getReceiveAddressListState({
        open: true,
        isAdmin: true,
        selectedNeedsAddress: true,
        addressWalletId: 'wallet-1',
      })
    ).toEqual({
      adminQuery: { wallet_id: 'wallet-1' },
      adminEnabled: true,
      walletEnabled: false,
    });
  });

  it('does not fetch addresses while the drawer is closed or address-free payloads are selected', () => {
    expect(
      getReceiveAddressListState({
        open: false,
        isAdmin: false,
        selectedNeedsAddress: true,
      })
    ).toEqual({
      adminQuery: undefined,
      adminEnabled: false,
      walletEnabled: false,
    });

    expect(
      getReceiveAddressListState({
        open: true,
        isAdmin: false,
        selectedNeedsAddress: false,
      })
    ).toEqual({
      adminQuery: undefined,
      adminEnabled: false,
      walletEnabled: false,
    });
  });
});
