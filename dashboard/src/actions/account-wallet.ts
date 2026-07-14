import type {
  ListAccountApiKeysData,
  ListWalletInvoicesData,
  ListWalletBtcAddressesData,
} from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { useAccountContext } from 'src/contexts/account';
import {
  listContacts,
  InvoiceOrderBy,
  OrderDirection,
  getWalletBalance,
  getWalletInvoice,
  getWalletPayment,
  getAccountAddress,
  listAccountApiKeys,
  listWalletInvoices,
  listWalletPayments,
  listWalletBtcAddresses,
} from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useActiveWallet() {
  const {
    activeWallet,
    activeWalletId,
    activeWalletLoading,
    activeWalletError,
    refreshActiveWallet,
  } = useAccountContext();

  return {
    wallet: activeWallet,
    walletId: activeWalletId,
    walletLoading: activeWalletLoading,
    walletError: activeWalletError,
    walletMutate: refreshActiveWallet,
  };
}

export function useGetWalletBalance(walletId?: string) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const selectedWalletId = walletId ?? activeWalletId;
  const key = selectedWalletId ? endpointKeys.accountWallet.balance(selectedWalletId) : null;

  const result = useSWR(key, () =>
    getWalletBalance<true>({ path: { wallet_id: selectedWalletId! } })
  );

  return useMemo(
    () => ({
      userBalance: result.data?.data,
      userBalanceLoading: walletsLoading || result.isLoading,
      userBalanceError: walletsError || result.error,
      userBalanceValidating: result.isValidating,
    }),
    [result, walletsError, walletsLoading]
  );
}

export function useListWalletInvoices(query?: ListWalletInvoicesData['query'], walletId?: string) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const selectedWalletId = walletId ?? activeWalletId;
  const key = selectedWalletId ? endpointKeys.accountWallet.invoices.list(selectedWalletId) : null;

  const result = useSWR(key, () =>
    listWalletInvoices<true>({
      path: { wallet_id: selectedWalletId! },
      query: {
        order_by: InvoiceOrderBy.CREATED_AT,
        order_direction: OrderDirection.DESC,
        ...query,
      },
    })
  );

  return useMemo(
    () => ({
      invoices: selectedWalletId ? result.data?.data : [],
      invoicesLoading: walletsLoading || result.isLoading,
      invoicesError: walletsError || result.error,
      invoicesValidating: result.isValidating,
      invoicesMutate: result.mutate,
    }),
    [result, selectedWalletId, walletsError, walletsLoading]
  );
}

export function useGetWalletInvoice(id: string, walletId?: string) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const selectedWalletId = walletId ?? activeWalletId;
  const key = selectedWalletId
    ? endpointKeys.accountWallet.invoices.get(selectedWalletId, id)
    : null;

  const result = useSWR(key, () =>
    getWalletInvoice<true>({ path: { wallet_id: selectedWalletId!, id } })
  );

  return {
    invoice: result.data?.data,
    invoiceLoading: walletsLoading || result.isLoading,
    invoiceError: walletsError || result.error,
    invoiceValidating: result.isValidating,
  };
}

export function useListWalletPayments(limit?: number, offset?: number, walletId?: string) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const selectedWalletId = walletId ?? activeWalletId;
  const key = selectedWalletId
    ? endpointKeys.accountWallet.payments.list(selectedWalletId, limit, offset)
    : null;

  const result = useSWR(key, () =>
    listWalletPayments<true>({
      path: { wallet_id: selectedWalletId! },
      query: { limit, offset, order_direction: OrderDirection.DESC },
    })
  );

  return useMemo(
    () => ({
      payments: selectedWalletId ? result.data?.data : [],
      paymentsLoading: walletsLoading || result.isLoading,
      paymentsError: walletsError || result.error,
      paymentsValidating: result.isValidating,
      paymentsMutate: result.mutate,
    }),
    [result, selectedWalletId, walletsError, walletsLoading]
  );
}

export function useGetWalletPayment(id: string, walletId?: string) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const selectedWalletId = walletId ?? activeWalletId;
  const key = selectedWalletId
    ? endpointKeys.accountWallet.payments.get(selectedWalletId, id)
    : null;

  const result = useSWR(key, () =>
    getWalletPayment<true>({ path: { wallet_id: selectedWalletId!, id } })
  );

  return {
    payment: result.data?.data,
    paymentLoading: walletsLoading || result.isLoading,
    paymentError: walletsError || result.error,
    paymentValidating: result.isValidating,
  };
}

export function useGetAccountLnAddress(shouldRetryOnError: boolean = false) {
  const result = useSWR(endpointKeys.account.lnAddress.get, () => getAccountAddress<true>(), {
    shouldRetryOnError,
  });

  return {
    lnAddress: result.data?.data,
    lnAddressLoading: result.isLoading,
    lnAddressError: result.error,
    lnAddressValidating: result.isValidating,
  };
}

type UseListWalletBtcAddressesOptions = {
  enabled?: boolean;
  walletId?: string;
};

export function useListWalletBtcAddresses(
  query?: ListWalletBtcAddressesData['query'],
  options?: UseListWalletBtcAddressesOptions
) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const enabled = options?.enabled ?? true;
  const selectedWalletId = options?.walletId ?? activeWalletId;
  const key =
    enabled && selectedWalletId
      ? [
          endpointKeys.accountWallet.btcAddresses.list,
          selectedWalletId,
          query?.limit,
          query?.offset,
          query?.address,
          query?.address_type,
          query?.used,
        ]
      : null;

  const result = useSWR(key, () =>
    listWalletBtcAddresses<true>({
      path: { wallet_id: selectedWalletId! },
      query: {
        limit: 50,
        order_direction: OrderDirection.DESC,
        ...query,
      },
    })
  );

  return useMemo(
    () => ({
      btcAddresses: selectedWalletId ? result.data?.data : [],
      btcAddressesLoading: walletsLoading || result.isLoading,
      btcAddressesError: walletsError || result.error,
      btcAddressesValidating: result.isValidating,
      btcAddressesMutate: result.mutate,
    }),
    [result, selectedWalletId, walletsError, walletsLoading]
  );
}

export function useListWalletContacts(walletId?: string) {
  const { activeWalletId, walletsLoading, walletsError } = useAccountContext();
  const selectedWalletId = walletId ?? activeWalletId;
  const key = selectedWalletId ? endpointKeys.accountWallet.contacts.list(selectedWalletId) : null;

  const result = useSWR(key, () => listContacts<true>({ path: { wallet_id: selectedWalletId! } }));

  return useMemo(
    () => ({
      contacts: selectedWalletId ? result.data?.data : [],
      contactsLoading: walletsLoading || result.isLoading,
      contactsError: walletsError || result.error,
      contactsValidating: result.isValidating,
    }),
    [result, selectedWalletId, walletsError, walletsLoading]
  );
}

export function useListAccountApiKeys(query?: ListAccountApiKeysData) {
  const result = useSWR(endpointKeys.account.apiKeys.list, () => listAccountApiKeys<true>(query));

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
