import type { ListWalletApiKeysData, ListWalletInvoicesData } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import {
  listContacts,
  getUserWallet,
  getWalletAddress,
  getWalletBalance,
  getWalletInvoice,
  getWalletPayment,
  listWalletApiKeys,
  listWalletInvoices,
  listWalletPayments,
} from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useGetUserWallet() {
  const result = useSWR(endpointKeys.userWallet.get, () => getUserWallet<true>());

  return useMemo(
    () => ({
      wallet: result.data?.data,
      walletLoading: result.isLoading,
      walletError: result.error,
      walletValidating: result.isValidating,
    }),
    [result]
  );
}

export function useGetWalletBalance() {
  const result = useSWR(endpointKeys.userWallet.balance, () => getWalletBalance<true>());

  return useMemo(
    () => ({
      userBalance: result.data?.data,
      userBalanceLoading: result.isLoading,
      userBalanceError: result.error,
      userBalanceValidating: result.isValidating,
    }),
    [result]
  );
}

export function useListWalletInvoices(query?: ListWalletInvoicesData) {
  const result = useSWR(endpointKeys.userWallet.invoices.list, () =>
    listWalletInvoices<true>(query)
  );

  return useMemo(
    () => ({
      invoices: result.data?.data,
      invoicesLoading: result.isLoading,
      invoicesError: result.error,
      invoicesValidating: result.isValidating,
      invoicesMutate: result.mutate,
    }),
    [result]
  );
}

export function useGetWalletInvoice(id: string) {
  const result = useSWR(endpointKeys.userWallet.invoices.get, () =>
    getWalletInvoice<true>({ path: { id } })
  );

  return {
    invoice: result.data?.data,
    invoiceLoading: result.isLoading,
    invoiceError: result.error,
    invoiceValidating: result.isValidating,
  };
}

export function useListWalletPayments(limit?: number, offset?: number) {
  const result = useSWR(endpointKeys.userWallet.payments.list, () =>
    listWalletPayments<true>({ query: { limit, offset } })
  );

  return useMemo(
    () => ({
      payments: result.data?.data,
      paymentsLoading: result.isLoading,
      paymentsError: result.error,
      paymentsValidating: result.isValidating,
      paymentsMutate: result.mutate,
    }),
    [result]
  );
}

export function useGetWalletPayment(id: string) {
  const result = useSWR(endpointKeys.userWallet.payments.get, () =>
    getWalletPayment<true>({ path: { id } })
  );

  return {
    payment: result.data?.data,
    paymentLoading: result.isLoading,
    paymentError: result.error,
    paymentValidating: result.isValidating,
  };
}

export function useGetWalletLnAddress(shouldRetryOnError: boolean = false) {
  const result = useSWR(endpointKeys.userWallet.lnAddress.get, () => getWalletAddress<true>(), {
    shouldRetryOnError,
  });

  return {
    lnAddress: result.data?.data,
    lnAddressLoading: result.isLoading,
    lnAddressError: result.error,
    lnAddressValidating: result.isValidating,
  };
}

export function useListWalletContacts() {
  const result = useSWR(endpointKeys.userWallet.contacts.list, () => listContacts<true>());

  return useMemo(
    () => ({
      contacts: result.data?.data,
      contactsLoading: result.isLoading,
      contactsError: result.error,
      contactsValidating: result.isValidating,
    }),
    [result]
  );
}

export function useListWalletApiKeys(query?: ListWalletApiKeysData) {
  const result = useSWR(endpointKeys.userWallet.apiKeys.list, () => listWalletApiKeys<true>(query));

  return useMemo(
    () => ({
      apiKeys: result.data?.data,
      apiKeysLoading: result.isLoading,
      apiKeysError: result.error,
      apiKeysValidating: result.isValidating,
    }),
    [result]
  );
}
