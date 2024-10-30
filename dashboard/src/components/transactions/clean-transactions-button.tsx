import type { LoadingButtonProps } from '@mui/lab';

import { LoadingButton } from '@mui/lab';

import { useBoolean } from 'src/hooks/use-boolean';

import { useTranslate } from 'src/locales';
import { deleteFailedPayments, deleteExpiredInvoices } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

interface Props {
  onSuccess: VoidFunction;
  buttonProps?: LoadingButtonProps;
  children?: React.ReactNode;
  transactionType?: TransactionType;
}

export function CleanTransactionsButton({ onSuccess, buttonProps, transactionType, children }: Props) {
  const { t } = useTranslate();
  const isDeleting = useBoolean();

  const handleCleanTransactions = async () => {
    try {
      isDeleting.onTrue();

      let nInvoicesDeleted = 0;
      let nPaymentsDeleted = 0;

      if (transactionType === TransactionType.INVOICE || !transactionType) {
        const { data } = await deleteExpiredInvoices();
        nInvoicesDeleted = data!;
      }

      if (transactionType === TransactionType.PAYMENT || !transactionType) {
        const { data } = await deleteFailedPayments();
        nPaymentsDeleted = data!;
      }

      if (nInvoicesDeleted > 0) {
        toast.success(t('clean_transactions_button.invoices_deleted_success', { count: nInvoicesDeleted }));
        onSuccess();
      }

      if (nPaymentsDeleted > 0) {
        toast.success(t('clean_transactions_button.payments_deleted_success', { count: nPaymentsDeleted }));
        onSuccess();
      }
    } catch (error) {
      toast.error(error.reason);
    } finally {
      isDeleting.onFalse();
    }
  };

  return (
    <LoadingButton loading={isDeleting.value} onClick={handleCleanTransactions} {...buttonProps}>
      {children || t('clean_transactions_button.clean_transactions')}{' '}
    </LoadingButton>
  );
}
