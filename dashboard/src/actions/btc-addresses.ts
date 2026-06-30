import type { ListBtcAddressesData } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { OrderDirection, listBtcAddresses } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

type UseListBtcAddressesOptions = {
  enabled?: boolean;
};

export function useListBtcAddresses(
  query?: ListBtcAddressesData['query'],
  options?: UseListBtcAddressesOptions
) {
  const enabled = options?.enabled ?? true;
  const key = enabled
    ? [
        endpointKeys.bitcoin.addresses.list,
        query?.wallet_id ?? 'all',
        query?.limit,
        query?.offset,
        query?.address,
        query?.address_type,
        query?.used,
      ]
    : null;

  const result = useSWR(key, () =>
    listBtcAddresses<true>({
      query: {
        limit: 50,
        order_direction: OrderDirection.DESC,
        ...query,
      },
    })
  );

  return useMemo(
    () => ({
      btcAddresses: result.data?.data,
      btcAddressesLoading: result.isLoading,
      btcAddressesError: result.error,
      btcAddressesValidating: result.isValidating,
      btcAddressesMutate: result.mutate,
    }),
    [result]
  );
}
