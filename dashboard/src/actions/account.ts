import useSWR from 'swr';
import { useMemo } from 'react';

import { listAccounts, getAccountById } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useListAccounts() {
  const result = useSWR(endpointKeys.accounts.list, () => listAccounts<true>());

  return useMemo(
    () => ({
      accounts: result.data?.data,
      accountsLoading: result.isLoading,
      accountsError: result.error,
      accountsValidating: result.isValidating,
    }),
    [result]
  );
}

export function useGetAccount(id: string) {
  const result = useSWR(endpointKeys.accounts.get(id), () =>
    getAccountById<true>({ path: { id } })
  );

  return useMemo(
    () => ({
      account: result.data?.data,
      accountLoading: result.isLoading,
      accountError: result.error,
      accountValidating: result.isValidating,
    }),
    [result]
  );
}
