import type { GetAccountSpendingReportData } from 'src/lib/swissknife';

import useSWR from 'swr';

import { getAccountSpendingReport } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

export type SpendingReportFilters = NonNullable<GetAccountSpendingReportData['query']>;

export function useSpendingReport(
  accountId: string,
  query: SpendingReportFilters,
  enabled: boolean
) {
  return useSWR(
    enabled ? [endpointKeys.accounts.spendingReport(accountId), query] : null,
    async () => {
      const response = await getAccountSpendingReport<true>({
        path: { account_id: accountId },
        query,
      });
      return response.data;
    },
    { refreshInterval: 15000 }
  );
}
