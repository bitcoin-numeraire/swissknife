'use client';

import type { Wallet, Account } from 'src/lib/swissknife';
import type { DashboardPreferenceUpdate } from './dashboard-preferences';

import { createContext } from 'react';

export type AccountContextValue = {
  account?: Account;
  wallets: Wallet[];
  activeWallet?: Wallet;
  activeWalletId?: string;
  accountLoading: boolean;
  walletsLoading: boolean;
  activeWalletLoading: boolean;
  accountError?: Error;
  walletsError?: Error;
  activeWalletError?: Error;
  selectWallet: (walletId: string) => Promise<void>;
  updateDashboardPreferences: (update: DashboardPreferenceUpdate) => Promise<void>;
  refreshAccount: () => Promise<unknown>;
  refreshWallets: () => Promise<unknown>;
  refreshActiveWallet: () => Promise<unknown>;
};

export const AccountContext = createContext<AccountContextValue | undefined>(undefined);
