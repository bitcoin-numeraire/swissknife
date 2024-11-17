import type { InvoiceResponse, ListInvoicesResponse } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { getInvoice, listInvoices } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

type IListInvoices = {
  invoices?: ListInvoicesResponse;
  invoicesLoading: boolean;
  invoicesError?: any;
  invoicesValidating: boolean;
};

export function useListInvoices(limit?: number, offset?: number): IListInvoices {
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

type IGetInvoice = {
  invoice?: InvoiceResponse;
  invoiceLoading: boolean;
  invoiceError?: any;
  invoiceValidating: boolean;
};

export function useGetInvoice(id: string): IGetInvoice {
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
