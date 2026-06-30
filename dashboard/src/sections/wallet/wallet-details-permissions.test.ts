import { it, expect, describe } from 'vitest';

import { Permission } from 'src/lib/swissknife';

import { getWalletDetailsPermissionState } from './wallet-details-permissions';

describe('getWalletDetailsPermissionState', () => {
  it('does not enable address modules for a wallet-only operator', () => {
    expect(
      getWalletDetailsPermissionState([Permission.READ_WALLET, Permission.WRITE_WALLET])
    ).toEqual({
      canReadBtcAddresses: false,
      canReadLnAddresses: false,
    });
  });

  it('enables optional address modules independently', () => {
    expect(
      getWalletDetailsPermissionState([Permission.READ_WALLET, Permission.READ_BTC_ADDRESS])
    ).toEqual({
      canReadBtcAddresses: true,
      canReadLnAddresses: false,
    });

    expect(
      getWalletDetailsPermissionState([Permission.READ_WALLET, Permission.READ_LN_ADDRESS])
    ).toEqual({
      canReadBtcAddresses: false,
      canReadLnAddresses: true,
    });
  });

  it('enables optional address modules when auth is skipped', () => {
    expect(getWalletDetailsPermissionState([], true)).toEqual({
      canReadBtcAddresses: true,
      canReadLnAddresses: true,
    });
  });
});
