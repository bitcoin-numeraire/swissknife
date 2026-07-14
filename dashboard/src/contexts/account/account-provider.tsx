'use client';

import type { Wallet, Account } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo, useState, useEffect, useCallback } from 'react';

import { endpointKeys } from 'src/actions/keys';
import {
  getAccount,
  getAccountWallet,
  listAccountWallets,
  updateAccountPreferences,
} from 'src/lib/swissknife';

import { AccountContext } from './account-context';
import { selectInitialWalletId, settingsWithActiveWallet } from './account-selection';

type AccountProviderProps = {
  children: React.ReactNode;
};

export function AccountProvider({ children }: AccountProviderProps) {
  const [activeWalletId, setActiveWalletId] = useState<string>();

  const accountResult = useSWR<Account>(endpointKeys.account.get, async () => {
    const response = await getAccount<true>();
    return response.data;
  });
  const walletsResult = useSWR<Wallet[]>(endpointKeys.account.wallets, async () => {
    const response = await listAccountWallets<true>();
    return response.data;
  });

  const account = accountResult.data;
  const wallets = useMemo(() => walletsResult.data ?? [], [walletsResult.data]);

  useEffect(() => {
    if (accountResult.isLoading || walletsResult.isLoading) return;

    setActiveWalletId((currentWalletId) =>
      selectInitialWalletId(wallets, currentWalletId, account?.preferences?.dashboard_settings)
    );
  }, [
    account?.preferences?.dashboard_settings,
    accountResult.isLoading,
    wallets,
    walletsResult.isLoading,
  ]);

  const activeWalletResult = useSWR<Wallet>(
    activeWalletId ? endpointKeys.accountWallet.get(activeWalletId) : null,
    async () => {
      const response = await getAccountWallet<true>({ path: { wallet_id: activeWalletId! } });
      return response.data;
    }
  );

  const selectWallet = useCallback(
    async (walletId: string) => {
      if (!wallets.some((wallet) => wallet.id === walletId)) {
        throw new Error('Wallet does not belong to the authenticated account.');
      }

      const previousWalletId = activeWalletId;
      const dashboardSettings = settingsWithActiveWallet(
        account?.preferences?.dashboard_settings,
        walletId
      );
      setActiveWalletId(walletId);

      try {
        const response = await updateAccountPreferences<true>({
          body: { dashboard_settings: dashboardSettings },
        });
        await accountResult.mutate(
          account
            ? {
                ...account,
                preferences: response.data,
              }
            : undefined,
          { revalidate: false }
        );
      } catch (error) {
        setActiveWalletId(previousWalletId);
        throw error;
      }
    },
    [account, accountResult, activeWalletId, wallets]
  );

  const value = useMemo(
    () => ({
      account,
      wallets,
      activeWallet: activeWalletResult.data,
      activeWalletId,
      accountLoading: accountResult.isLoading,
      walletsLoading: walletsResult.isLoading,
      activeWalletLoading:
        accountResult.isLoading || walletsResult.isLoading || activeWalletResult.isLoading,
      accountError: accountResult.error,
      walletsError: walletsResult.error,
      activeWalletError: activeWalletResult.error,
      selectWallet,
      refreshAccount: accountResult.mutate,
      refreshWallets: walletsResult.mutate,
      refreshActiveWallet: activeWalletResult.mutate,
    }),
    [
      account,
      wallets,
      activeWalletResult.data,
      activeWalletId,
      accountResult.isLoading,
      walletsResult.isLoading,
      activeWalletResult.isLoading,
      accountResult.error,
      walletsResult.error,
      activeWalletResult.error,
      selectWallet,
      accountResult.mutate,
      walletsResult.mutate,
      activeWalletResult.mutate,
    ]
  );

  return <AccountContext value={value}>{children}</AccountContext>;
}
