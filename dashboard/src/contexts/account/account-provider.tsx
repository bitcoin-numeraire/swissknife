'use client';

import type { Wallet, Account } from 'src/lib/swissknife';
import type { DashboardPreferenceUpdate } from './dashboard-preferences';

import useSWR from 'swr';
import { useRef, useMemo, useState, useEffect, useCallback } from 'react';

import { useColorScheme } from '@mui/material/styles';

import { endpointKeys } from 'src/actions/keys';
import { getAccount, getAccountWallet, patchAccountPreferences } from 'src/lib/swissknife';

import { defaultSettings, useSettingsContext } from 'src/components/settings';

import { AccountContext } from './account-context';
import { useAccountEventStream } from './account-event-stream';
import { useAccountEventNotifications } from './account-event-notifications';
import { dashboardPreferencePatch, uiSettingsFromDashboardSettings } from './dashboard-preferences';
import {
  dashboardSettings,
  selectInitialWalletId,
  ACTIVE_WALLET_SETTING,
} from './account-selection';

type AccountProviderProps = {
  children: React.ReactNode;
};

export function AccountProvider({ children }: AccountProviderProps) {
  const [activeWalletId, setActiveWalletId] = useState<string>();
  const hydrated = useRef<{ id: string; settings: unknown } | undefined>(undefined);
  const { setMode } = useColorScheme();
  const { setState: setSettingsState } = useSettingsContext();

  const accountResult = useSWR<Account>(endpointKeys.account.get, async () => {
    const response = await getAccount<true>();
    return response.data;
  });
  const account = accountResult.data;
  const wallets = useMemo(() => account?.wallets ?? [], [account?.wallets]);
  const recentClientEvents = useAccountEventStream(account?.id);
  useAccountEventNotifications(account?.id, recentClientEvents, wallets);

  useEffect(() => {
    if (!account) {
      hydrated.current = undefined;
      return;
    }
    const stored = account.preferences?.dashboard_settings;
    if (hydrated.current?.id === account.id && hydrated.current.settings === stored) return;

    const persistedSettings = uiSettingsFromDashboardSettings(stored, defaultSettings);
    hydrated.current = { id: account.id, settings: stored };
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

  const saveSettings = useCallback(
    async (patch: Record<string, unknown>) => {
      if (!account) throw new Error('Account unavailable.');
      await accountResult.mutate(
        async (current) => {
          if (!current || current.id !== account?.id) throw new Error('Account changed.');
          const response = await patchAccountPreferences<true>({
            body: { dashboard_settings: patch },
          });
          return { ...current, preferences: response.data };
        },
        {
          optimisticData: (current) =>
            current?.preferences && current.id === account?.id
              ? {
                  ...current,
                  preferences: {
                    ...current.preferences,
                    dashboard_settings: {
                      ...dashboardSettings(current.preferences.dashboard_settings),
                      ...patch,
                    },
                  },
                }
              : (current ?? account),
          rollbackOnError: true,
          revalidate: true,
        }
      );
    },
    [account, accountResult]
  );

  const updateDashboardPreferences = useCallback(
    async (update: DashboardPreferenceUpdate) => {
      if (account) {
        await saveSettings(dashboardPreferencePatch(update));
      } else {
        setSettingsState(update);
        if (update.mode) setMode(update.mode);
      }
    },
    [account, saveSettings, setMode, setSettingsState]
  );

  const selectWallet = useCallback(
    async (walletId: string) => {
      if (!wallets.some((wallet) => wallet.id === walletId)) {
        throw new Error('Wallet does not belong to the authenticated account.');
      }

      await saveSettings({ [ACTIVE_WALLET_SETTING]: walletId });
      setActiveWalletId(walletId);
    },
    [saveSettings, wallets]
  );

  const value = useMemo(
    () => ({
      account,
      wallets,
      activeWallet: activeWalletResult.data,
      activeWalletId,
      recentClientEvents,
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
      recentClientEvents,
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
