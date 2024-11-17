import type {
  Balance,
  LnAddress,
  InvoiceResponse,
  PaymentResponse,
  ListContactsResponse,
  ListWalletApiKeysData,
  ListWalletInvoicesData,
  ListWalletApiKeysResponse,
  ListWalletInvoicesResponse,
  ListWalletPaymentsResponse,
} from 'src/lib/swissknife';

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

import type { IGetWallet } from './wallet';

// ----------------------------------------------------------------------

export function useGetUserWallet(): IGetWallet {
  const fetcher = async () => {
    const { data, error } = await getUserWallet();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.get, fetcher);

  return useMemo(
    () => ({
      wallet: data,
      walletLoading: isLoading,
      walletError: error,
      walletValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

type IGetUserBalance = {
  userBalance?: Balance;
  userBalanceLoading: boolean;
  userBalanceError?: any;
  userBalanceValidating: boolean;
};

export function useGetWalletBalance(): IGetUserBalance {
  const fetcher = async () => {
    const { data, error } = await getWalletBalance();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.balance, fetcher);

  return useMemo(
    () => ({
      userBalance: data,
      userBalanceLoading: isLoading,
      userBalanceError: error,
      userBalanceValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

type IListInvoices = {
  invoices?: ListWalletInvoicesResponse;
  invoicesLoading: boolean;
  invoicesError?: any;
  invoicesValidating: boolean;
};

export function useListWalletInvoices(query?: ListWalletInvoicesData): IListInvoices {
  const fetcher = async () => {
    const { data, error } = await listWalletInvoices(query);
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.invoices.list, fetcher);

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

export function useGetWalletInvoice(id: string): IGetInvoice {
  const fetcher = async () => {
    const { data, error } = await getWalletInvoice({ path: { id } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.invoices.get, fetcher);

  return {
    invoice: data,
    invoiceLoading: isLoading,
    invoiceError: error,
    invoiceValidating: isValidating,
  };
}

type IListPayments = {
  payments?: ListWalletPaymentsResponse;
  paymentsLoading: boolean;
  paymentsError?: any;
  paymentsValidating: boolean;
};

export function useListWalletPayments(limit?: number, offset?: number): IListPayments {
  const fetcher = async () => {
    const { data, error } = await listWalletPayments({ query: { limit, offset } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.payments.list, fetcher);

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

type IGetPayment = {
  payment?: PaymentResponse;
  paymentLoading: boolean;
  paymentError?: any;
  paymentValidating: boolean;
};

export function useGetWalletPayment(id: string): IGetPayment {
  const fetcher = async () => {
    const { data, error } = await getWalletPayment({ path: { id } });
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.payments.get, fetcher);

  return {
    payment: data,
    paymentLoading: isLoading,
    paymentError: error,
    paymentValidating: isValidating,
  };
}

type IGetLnAddress = {
  lnAddress?: LnAddress;
  lnAddressLoading: boolean;
  lnAddressError?: any;
  lnAddressValidating: boolean;
};

export function useGetWalletLnAddress(shouldRetryOnError: boolean = false): IGetLnAddress {
  const fetcher = async () => {
    const { data, error, response } = await getWalletAddress();
    if (error) {
      if (response.status === 404) {
        return undefined;
      }

      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.lnAddress.get, fetcher, {
    shouldRetryOnError,
  });

  return {
    lnAddress: data,
    lnAddressLoading: isLoading,
    lnAddressError: error,
    lnAddressValidating: isValidating,
  };
}

type IListContacts = {
  contacts?: ListContactsResponse;
  contactsLoading: boolean;
  contactsError?: any;
  contactsValidating: boolean;
};

export function useListWalletContacts(): IListContacts {
  const fetcher = async () => {
    const { data, error } = await listContacts();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.contacts.list, fetcher);

  return useMemo(
    () => ({
      contacts: data,
      contactsLoading: isLoading,
      contactsError: error,
      contactsValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

type IListApiKeys = {
  apiKeys?: ListWalletApiKeysResponse;
  apiKeysLoading: boolean;
  apiKeysError?: any;
  apiKeysValidating: boolean;
};

export function useListWalletApiKeys(query?: ListWalletApiKeysData): IListApiKeys {
  const fetcher = async () => {
    const { data, error } = await listWalletApiKeys(query);
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, error, isLoading, isValidating } = useSWR(endpointKeys.userWallet.apiKeys.list, fetcher);

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
