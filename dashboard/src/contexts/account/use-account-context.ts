'use client';

import { use } from 'react';

import { AccountContext } from './account-context';

export function useAccountContext() {
  const context = use(AccountContext);

  if (!context) throw new Error('useAccountContext must be used inside AccountProvider');

  return context;
}
