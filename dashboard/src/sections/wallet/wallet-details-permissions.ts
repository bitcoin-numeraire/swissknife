import { Permission } from 'src/lib/swissknife';

import { hasAllPermissions } from 'src/auth/permissions';

export function getWalletDetailsPermissionState(
  userPermissions: string[] = [],
  authSkip: boolean = false
) {
  return {
    canReadBtcAddresses:
      authSkip || hasAllPermissions([Permission.READ_BTC_ADDRESS], userPermissions),
    canReadLnAddresses:
      authSkip || hasAllPermissions([Permission.READ_LN_ADDRESS], userPermissions),
  };
}
