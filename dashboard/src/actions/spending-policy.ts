import useSWR from 'swr';

import { getAgentSpendingHours, getAccountSpendingPolicy } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

export function useSpendingPolicy(accountId: string) {
  return useSWR(
    endpointKeys.accounts.spendingPolicy(accountId),
    async () => {
      const result = await getAccountSpendingPolicy({
        path: { account_id: accountId },
        throwOnError: false,
      });
      if (result.response?.status === 404) return null;
      if (!result.data) throw result.error || new Error('Unable to load spending policy');
      return result.data;
    },
    { refreshInterval: 15000 }
  );
}

export function useAgentHours(accountId: string) {
  return useSWR(
    endpointKeys.accounts.agentHours(accountId),
    async () => {
      const result = await getAgentSpendingHours({
        path: { account_id: accountId },
        throwOnError: false,
      });
      if (result.response?.status === 404) return null;
      if (!result.data) throw result.error || new Error('Unable to load delegated spending hours');
      return result.data;
    },
    { refreshInterval: 15000 }
  );
}
