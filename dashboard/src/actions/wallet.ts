import type { WalletResponse, ListWalletOverviewsResponse } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { getWallet, listWalletOverviews } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export type IGetWallet = {
  wallet?: WalletResponse;
  walletLoading: boolean;
  walletError?: any;
  walletValidating: boolean;
};

export function useGetWallet(id: string): IGetWallet {
  const fetcher = async () => {
    const { data, error } = await getWallet({ path: { id } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.wallets.get, fetcher);

  return useMemo(
    () => ({
      wallet: data,
      walletLoading: isLoading,
      walletError: error,
      walletValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

type IListWallets = {
  walletOverviews?: ListWalletOverviewsResponse;
  walletOverviewsLoading: boolean;
  walletOverviewsError?: any;
  walletOverviewsValidating: boolean;
};

export function useListWalletOverviews(): IListWallets {
  const fetcher = async () => {
    const { data, error } = await listWalletOverviews();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.wallets.listOverviews, fetcher);

  return useMemo(
    () => ({
      walletOverviews: data,
      walletOverviewsLoading: isLoading,
      walletOverviewsError: error,
      walletOverviewsValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}
