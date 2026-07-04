import type {
  ListWalletApiKeysData,
  ListWalletInvoicesData,
  ListWalletBtcAddressesData,
} from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import {
  listContacts,
  InvoiceOrderBy,
  OrderDirection,
  getWalletAddress,
  getWalletBalance,
  getWalletInvoice,
  getWalletPayment,
  getAccountWallet,
  listWalletApiKeys,
  listAccountWallets,
  listWalletInvoices,
  listWalletPayments,
  listWalletBtcAddresses,
} from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useGetUserWallet() {
  const result = useSWR(endpointKeys.userWallet.get, async () => {
    const wallets = await listAccountWallets<true>({ query: { limit: 1 } });
    const wallet = wallets.data?.[0];

    if (!wallet) {
      return undefined;
    }

    const hydratedWallet = await getAccountWallet<true>({ path: { wallet_id: wallet.id } });
    return hydratedWallet.data;
  });

  return useMemo(
    () => ({
      wallet: result.data,
      walletLoading: result.isLoading,
      walletError: result.error,
      walletValidating: result.isValidating,
    }),
    [result]
  );
}

export function useGetWalletBalance(walletId?: string) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const activeWalletId = walletId ?? wallet?.id;
  const key = activeWalletId ? endpointKeys.userWallet.balance(activeWalletId) : null;

  const result = useSWR(key, () =>
    getWalletBalance<true>({ path: { wallet_id: activeWalletId! } })
  );

  return useMemo(
    () => ({
      userBalance: result.data?.data,
      userBalanceLoading: walletLoading || result.isLoading,
      userBalanceError: walletError || result.error,
      userBalanceValidating: result.isValidating,
    }),
    [result, walletError, walletLoading]
  );
}

export function useListWalletInvoices(query?: ListWalletInvoicesData['query'], walletId?: string) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const activeWalletId = walletId ?? wallet?.id;
  const key = activeWalletId ? endpointKeys.userWallet.invoices.list(activeWalletId) : null;

  const result = useSWR(key, () =>
    listWalletInvoices<true>({
      path: { wallet_id: activeWalletId! },
      query: {
        order_by: InvoiceOrderBy.CREATED_AT,
        order_direction: OrderDirection.DESC,
        ...query,
      },
    })
  );

  return useMemo(
    () => ({
      invoices: result.data?.data,
      invoicesLoading: walletLoading || result.isLoading,
      invoicesError: walletError || result.error,
      invoicesValidating: result.isValidating,
      invoicesMutate: result.mutate,
    }),
    [result, walletError, walletLoading]
  );
}

export function useGetWalletInvoice(id: string, walletId?: string) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const activeWalletId = walletId ?? wallet?.id;
  const key = activeWalletId ? endpointKeys.userWallet.invoices.get(activeWalletId, id) : null;

  const result = useSWR(key, () =>
    getWalletInvoice<true>({ path: { wallet_id: activeWalletId!, id } })
  );

  return {
    invoice: result.data?.data,
    invoiceLoading: walletLoading || result.isLoading,
    invoiceError: walletError || result.error,
    invoiceValidating: result.isValidating,
  };
}

export function useListWalletPayments(limit?: number, offset?: number, walletId?: string) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const activeWalletId = walletId ?? wallet?.id;
  const key = activeWalletId
    ? endpointKeys.userWallet.payments.list(activeWalletId, limit, offset)
    : null;

  const result = useSWR(key, () =>
    listWalletPayments<true>({
      path: { wallet_id: activeWalletId! },
      query: { limit, offset, order_direction: OrderDirection.DESC },
    })
  );

  return useMemo(
    () => ({
      payments: result.data?.data,
      paymentsLoading: walletLoading || result.isLoading,
      paymentsError: walletError || result.error,
      paymentsValidating: result.isValidating,
      paymentsMutate: result.mutate,
    }),
    [result, walletError, walletLoading]
  );
}

export function useGetWalletPayment(id: string, walletId?: string) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const activeWalletId = walletId ?? wallet?.id;
  const key = activeWalletId ? endpointKeys.userWallet.payments.get(activeWalletId, id) : null;

  const result = useSWR(key, () =>
    getWalletPayment<true>({ path: { wallet_id: activeWalletId!, id } })
  );

  return {
    payment: result.data?.data,
    paymentLoading: walletLoading || result.isLoading,
    paymentError: walletError || result.error,
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

type UseListWalletBtcAddressesOptions = {
  enabled?: boolean;
  walletId?: string;
};

export function useListWalletBtcAddresses(
  query?: ListWalletBtcAddressesData['query'],
  options?: UseListWalletBtcAddressesOptions
) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const enabled = options?.enabled ?? true;
  const activeWalletId = options?.walletId ?? wallet?.id;
  const key =
    enabled && activeWalletId
      ? [
          endpointKeys.userWallet.btcAddresses.list,
          activeWalletId,
          query?.limit,
          query?.offset,
          query?.address,
          query?.address_type,
          query?.used,
        ]
      : null;

  const result = useSWR(key, () =>
    listWalletBtcAddresses<true>({
      path: { wallet_id: activeWalletId! },
      query: {
        limit: 50,
        order_direction: OrderDirection.DESC,
        ...query,
      },
    })
  );

  return useMemo(
    () => ({
      btcAddresses: result.data?.data,
      btcAddressesLoading: walletLoading || result.isLoading,
      btcAddressesError: walletError || result.error,
      btcAddressesValidating: result.isValidating,
      btcAddressesMutate: result.mutate,
    }),
    [result, walletError, walletLoading]
  );
}

export function useListWalletContacts(walletId?: string) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const activeWalletId = walletId ?? wallet?.id;
  const key = activeWalletId ? endpointKeys.userWallet.contacts.list(activeWalletId) : null;

  const result = useSWR(key, () => listContacts<true>({ path: { wallet_id: activeWalletId! } }));

  return useMemo(
    () => ({
      contacts: result.data?.data,
      contactsLoading: walletLoading || result.isLoading,
      contactsError: walletError || result.error,
      contactsValidating: result.isValidating,
    }),
    [result, walletError, walletLoading]
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
