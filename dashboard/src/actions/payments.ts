import type { PaymentResponse, ListPaymentsResponse } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { getPayment, listPayments } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

interface IGetPayments {
  payments?: ListPaymentsResponse;
  paymentsLoading: boolean;
  paymentsError?: any;
  paymentsValidating: boolean;
}

export function useListPayments(limit?: number, offset?: number): IGetPayments {
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

interface IGetPayment {
  payment?: PaymentResponse;
  paymentLoading: boolean;
  paymentError?: any;
  paymentValidating: boolean;
}

export function useGetPayment(id: string): IGetPayment {
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
