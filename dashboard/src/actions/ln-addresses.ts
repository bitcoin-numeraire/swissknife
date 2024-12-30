import useSWR from 'swr';
import { useMemo } from 'react';

import { getAddress, listAddresses } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

export function useListLnAddresses(limit?: number, offset?: number) {
  const fetcher = async () => {
    const { data, error } = await listAddresses({ query: { limit, offset } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(
    endpointKeys.lightning.addresses.list,
    fetcher
  );

  return useMemo(
    () => ({
      lnAddresses: data,
      lnAddressesLoading: isLoading,
      lnAddressesError: error,
      lnAddressesValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

export function useGetLnAddress(id: string) {
  const fetcher = async () => {
    const { data, error } = await getAddress({ path: { id } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(
    endpointKeys.lightning.addresses.get,
    fetcher
  );

  return {
    lnAddress: data,
    lnAddressLoading: isLoading,
    lnAddressError: error,
    lnAddressValidating: isValidating,
  };
}
