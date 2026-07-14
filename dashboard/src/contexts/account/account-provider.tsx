'use client';

import type { Wallet, Account } from 'src/lib/swissknife';
import type { DashboardPreferenceUpdate } from './dashboard-preferences';

import useSWR from 'swr';
import { useRef, useMemo, useState, useEffect, useCallback } from 'react';

import { useColorScheme } from '@mui/material/styles';

import { endpointKeys } from 'src/actions/keys';
import { getAccount, getAccountWallet, updateAccountPreferences } from 'src/lib/swissknife';

import { defaultSettings, useSettingsContext } from 'src/components/settings';

import { AccountContext } from './account-context';
import { selectInitialWalletId, settingsWithActiveWallet } from './account-selection';
import {
  settingsWithUiPreferences,
  uiSettingsFromDashboardSettings,
} from './dashboard-preferences';

type AccountProviderProps = {
  children: React.ReactNode;
};

export function AccountProvider({ children }: AccountProviderProps) {
  const [activeWalletId, setActiveWalletId] = useState<string>();
  const hydratedAccountId = useRef<string | undefined>(undefined);
  const { setMode } = useColorScheme();
  const { state: settingsState, setState: setSettingsState } = useSettingsContext();

  const accountResult = useSWR<Account>(endpointKeys.account.get, async () => {
    const response = await getAccount<true>();
    return response.data;
  });
  const account = accountResult.data;
  const wallets = useMemo(() => account?.wallets ?? [], [account?.wallets]);

  useEffect(() => {
    if (!account || hydratedAccountId.current === account.id) return;

    const persistedSettings = uiSettingsFromDashboardSettings(
      account.preferences?.dashboard_settings,
      defaultSettings
    );
    hydratedAccountId.current = account.id;
    setSettingsState(persistedSettings);
    setMode(persistedSettings.mode ?? defaultSettings.mode ?? 'system');
  }, [account, setMode, setSettingsState]);

  useEffect(() => {
    if (accountResult.isLoading) return;

    setActiveWalletId((currentWalletId) =>
      selectInitialWalletId(wallets, currentWalletId, account?.preferences?.dashboard_settings)
    );
  }, [account?.preferences?.dashboard_settings, accountResult.isLoading, wallets]);

  const activeWalletResult = useSWR<Wallet>(
    activeWalletId ? endpointKeys.accountWallet.get(activeWalletId) : null,
    async () => {
      const response = await getAccountWallet<true>({ path: { wallet_id: activeWalletId! } });
      return response.data;
    }
  );

  const updateDashboardPreferences = useCallback(
    async (update: DashboardPreferenceUpdate) => {
      const previousSettings = settingsState;
      const nextSettings = { ...settingsState, ...update };
      setSettingsState(update);
      if (update.mode) setMode(update.mode);

      try {
        const response = await updateAccountPreferences<true>({
          body: {
            dashboard_settings: settingsWithUiPreferences(
              account?.preferences?.dashboard_settings,
              nextSettings
            ),
          },
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
        setSettingsState(previousSettings);
        if (update.mode) setMode(previousSettings.mode ?? 'system');
        throw error;
      }
    },
    [account, accountResult, setMode, setSettingsState, settingsState]
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
      walletsLoading: accountResult.isLoading,
      activeWalletLoading: accountResult.isLoading || activeWalletResult.isLoading,
      accountError: accountResult.error,
      walletsError: accountResult.error,
      activeWalletError: activeWalletResult.error,
      selectWallet,
      updateDashboardPreferences,
      refreshAccount: accountResult.mutate,
      refreshWallets: accountResult.mutate,
      refreshActiveWallet: activeWalletResult.mutate,
    }),
    [
      account,
      wallets,
      activeWalletResult.data,
      activeWalletId,
      accountResult.isLoading,
      activeWalletResult.isLoading,
      accountResult.error,
      activeWalletResult.error,
      selectWallet,
      updateDashboardPreferences,
      accountResult.mutate,
      activeWalletResult.mutate,
    ]
  );

  return <AccountContext value={value}>{children}</AccountContext>;
}
