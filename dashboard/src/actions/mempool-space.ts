import type { IFiatPrices } from 'src/types/bitcoin';

import useSWR from 'swr';
import { useMemo } from 'react';

import { CONFIG } from 'src/global-config';

import { endpointKeys } from './keys';

export function useFetchFiatPrices() {
  const fetcher = async (): Promise<IFiatPrices> => {
    const data = await fetch(`${CONFIG.mempoolSpace}/prices`);
    return data.json();
  };

  const { data, isLoading, error, isValidating } = useSWR(
    endpointKeys.mempoolSpace.prices,
    fetcher
  );

  return useMemo(
    () => ({
      fiatPrices: data,
      fiatPricesLoading: isLoading,
      fiatPricesError: error,
      fiatPricesValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}
