import useSWR from 'swr';
import { useMemo } from 'react';

import { getWallet, listWallets, listWalletOverviews } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useGetWallet(id: string) {
  const result = useSWR(endpointKeys.wallets.get, () => getWallet<true>({ path: { id } }));

  return useMemo(
    () => ({
      wallet: result.data?.data,
      walletLoading: result.isLoading,
      walletError: result.error,
      walletValidating: result.isValidating,
    }),
    [result]
  );
}

export function useListWallets() {
  const result = useSWR(endpointKeys.wallets.list, () => listWallets<true>());

  return useMemo(
    () => ({
      wallets: result.data?.data,
      walletsLoading: result.isLoading,
      walletsError: result.error,
      walletsValidating: result.isValidating,
    }),
    [result]
  );
}

export function useListWalletOverviews() {
  const result = useSWR(endpointKeys.wallets.listOverviews, () => listWalletOverviews<true>());

  return useMemo(
    () => ({
      walletOverviews: result.data?.data,
      walletOverviewsLoading: result.isLoading,
      walletOverviewsError: result.error,
      walletOverviewsValidating: result.isValidating,
    }),
    [result]
  );
}
