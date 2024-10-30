import type { IFiatPrices } from 'src/types/bitcoin';

import useSWR from 'swr';
import { useMemo } from 'react';

import { CONFIG } from 'src/config-global';

import { endpointKeys } from './keys';

interface IGetFiatPrices {
  fiatPrices?: IFiatPrices;
  fiatPricesLoading: boolean;
  fiatPricesError: any;
  fiatPricesValidating: boolean;
}

export function useFetchFiatPrices(): IGetFiatPrices {
  const fetcher = async (): Promise<IFiatPrices> => {
    const data = await fetch(`${CONFIG.site.mempoolSpace}/prices`);
    return data.json();
  };

  const { data, isLoading, error, isValidating } = useSWR(endpointKeys.mempoolSpace.prices, fetcher);

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
