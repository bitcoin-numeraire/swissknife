import type { ListApiKeysResponse } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { listApiKeys } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

type IListApiKeys = {
  apiKeys?: ListApiKeysResponse;
  apiKeysLoading: boolean;
  apiKeysError?: any;
  apiKeysValidating: boolean;
};

export function useListApiKeys(): IListApiKeys {
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
