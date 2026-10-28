import type { ButtonProps } from '@mui/material/Button';

import { useBoolean } from 'minimal-shared/hooks';

import Button from '@mui/material/Button';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import {
  InvoiceStatus,
  PaymentStatus,
  deleteInvoices,
  deletePayments,
  deleteFailedPayments,
  deleteExpiredInvoices,
} from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

interface Props {
  scope: 'wallet' | 'admin';
  walletId?: string;
  onSuccess: VoidFunction;
  buttonProps?: ButtonProps;
  children?: React.ReactNode;
  transactionType?: TransactionType;
}

export function CleanTransactionsButton({
  scope,
  walletId,
  onSuccess,
  buttonProps,
  transactionType,
  children,
}: Props) {
  const { t } = useTranslate();
  const isDeleting = useBoolean();

  const handleCleanTransactions = async () => {
    if (scope === 'wallet' && !walletId) return;

    let shouldRefresh = false;

    try {
      isDeleting.onTrue();

      let nInvoicesDeleted = 0;
      let nPaymentsDeleted = 0;

      if (transactionType === TransactionType.INVOICE || !transactionType) {
        const { data } =
          scope === 'admin'
            ? await deleteInvoices<true>({
                query: { status: InvoiceStatus.EXPIRED },
                throwOnError: true,
              })
            : await deleteExpiredInvoices<true>({
                path: { wallet_id: walletId! },
                throwOnError: true,
              });
        nInvoicesDeleted = data;
        shouldRefresh = true;

        if (nInvoicesDeleted > 0) {
          toast.success(
            t('clean_transactions_button.invoices_deleted_success', { count: nInvoicesDeleted })
          );
        }
      }

      if (transactionType === TransactionType.PAYMENT || !transactionType) {
        const { data } =
          scope === 'admin'
            ? await deletePayments<true>({
                query: { status: PaymentStatus.FAILED },
                throwOnError: true,
              })
            : await deleteFailedPayments<true>({
                path: { wallet_id: walletId! },
                throwOnError: true,
              });
        nPaymentsDeleted = data;
        shouldRefresh = true;

        if (nPaymentsDeleted > 0) {
          toast.success(
            t('clean_transactions_button.payments_deleted_success', { count: nPaymentsDeleted })
          );
        }
      }

      if (nInvoicesDeleted === 0 && nPaymentsDeleted === 0) {
        toast.info(t('clean_transactions_button.nothing_to_clean'));
      }
    } catch (error) {
      handleActionError(error);
    } finally {
      isDeleting.onFalse();
      if (shouldRefresh) onSuccess();
    }
  };

  return (
    <Button
      {...buttonProps}
      loading={isDeleting.value}
      onClick={handleCleanTransactions}
      disabled={buttonProps?.disabled || (scope === 'wallet' && !walletId)}
    >
      {children || t('clean_transactions_button.clean_transactions')}
    </Button>
  );
}
