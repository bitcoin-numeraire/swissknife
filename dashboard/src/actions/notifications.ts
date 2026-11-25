import useSWR from 'swr';

import { listAccountNotifications } from 'src/lib/swissknife';

export const notificationKey = 'accountNotifications';

export function isNotificationKey(key: unknown, accountId: string | undefined) {
  return (
    Boolean(accountId) && Array.isArray(key) && key[0] === notificationKey && key[1] === accountId
  );
}

export function useNotificationPage(
  accountId: string | undefined,
  archived = false,
  before?: number
) {
  return useSWR(accountId ? [notificationKey, accountId, archived, before] : null, async () => {
    const { data, error } = await listAccountNotifications({
      query: { archived, before, limit: 20 },
    });
    if (error) throw new Error(error.reason);
    return data;
  });
}
