import useSWR from 'swr';
import { useMemo } from 'react';

import { getInvoice, listInvoices } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useListInvoices(limit?: number, offset?: number) {
  const fetcher = async () => {
    const { data, error } = await listInvoices({ query: { limit, offset } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.invoices.list, fetcher);

  return useMemo(
    () => ({
      invoices: data,
      invoicesLoading: isLoading,
      invoicesError: error,
      invoicesValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

export function useGetInvoice(id: string) {
  const fetcher = async () => {
    const { data, error } = await getInvoice({ path: { id } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.invoices.get, fetcher);

  return {
    invoice: data,
    invoiceLoading: isLoading,
    invoiceError: error,
    invoiceValidating: isValidating,
  };
}
