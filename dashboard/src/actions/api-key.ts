import useSWR from 'swr';
import { useMemo } from 'react';

import { listApiKeys } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useListApiKeys() {
  const fetcher = async () => {
    const { data, error } = await listApiKeys();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.apiKeys.list, fetcher);

  return useMemo(
    () => ({
      apiKeys: data,
      apiKeysLoading: isLoading,
      apiKeysError: error,
      apiKeysValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}
