import type { ListBtcAddressesData } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { OrderDirection, listBtcAddresses } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useListBtcAddresses(query?: ListBtcAddressesData['query']) {
  const key = query?.wallet_id
    ? [endpointKeys.bitcoin.addresses.list, query.wallet_id, query.limit, query.offset]
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
