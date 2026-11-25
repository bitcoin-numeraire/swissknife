import type { PaymentSubmission } from 'src/lib/swissknife';

import useSWR from 'swr';

import { getPaymentSubmission, getWalletPaymentSubmission } from 'src/lib/swissknife';

export const paymentSubmissionKey = 'accountWalletPaymentSubmission';

export function usePaymentSubmission(
  accountId: string | undefined,
  walletId: string | undefined,
  isAdmin: boolean,
  receipt: PaymentSubmission | undefined,
  paymentId?: string
) {
  const id = receipt?.payment.id ?? paymentId;
  return useSWR(
    id && accountId && walletId ? [paymentSubmissionKey, walletId, id, accountId, isAdmin] : null,
    async () => {
      const result = isAdmin
        ? await getPaymentSubmission({ path: { id: id! } })
        : await getWalletPaymentSubmission({
            path: { wallet_id: walletId!, id: id! },
          });
      // Older synchronous payments have no submission resource.
      if (result.response?.status === 404) return null;
      if (result.error) throw result.error;
      return result.data;
    },
    {
      fallbackData: receipt,
      // An operator can submit for another account whose SSE stream they cannot read.
      refreshInterval: (current) => (current?.payment.status === 'Pending' ? 5000 : 0),
    }
  );
}
