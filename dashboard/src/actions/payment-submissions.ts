import type { PaymentSubmission } from 'src/lib/swissknife';

import useSWR from 'swr';

import { getPaymentSubmission, getWalletPaymentSubmission } from 'src/lib/swissknife';

export const paymentSubmissionKey = 'accountWalletPaymentSubmission';

export function usePaymentSubmission(
  accountId: string | undefined,
  walletId: string | undefined,
  isAdmin: boolean,
  receipt: PaymentSubmission | undefined
) {
  const id = receipt?.payment.id;
  return useSWR(
    id && accountId && walletId ? [paymentSubmissionKey, walletId, id, accountId, isAdmin] : null,
    async () => {
      const { data } = isAdmin
        ? await getPaymentSubmission({ path: { id: id! }, throwOnError: true })
        : await getWalletPaymentSubmission({
            path: { wallet_id: walletId!, id: id! },
            throwOnError: true,
          });
      return data;
    },
    {
      fallbackData: receipt,
      // An operator can submit for another account whose SSE stream they cannot read.
      refreshInterval: (current) => (current?.payment.status === 'Pending' ? 5000 : 0),
    }
  );
}
