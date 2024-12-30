import useSWR from 'swr';
import { useMemo } from 'react';

import { getPayment, listPayments } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useListPayments(limit?: number, offset?: number) {
  const fetcher = async () => {
    const { data, error } = await listPayments({ query: { limit, offset } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.payments.list, fetcher);

  return useMemo(
    () => ({
      payments: data,
      paymentsLoading: isLoading,
      paymentsError: error,
      paymentsValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

export function useGetPayment(id: string) {
  const fetcher = async () => {
    const { data, error } = await getPayment({ path: { id } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.payments.get, fetcher);

  return {
    payment: data,
    paymentLoading: isLoading,
    paymentError: error,
    paymentValidating: isValidating,
  };
}
