import type { ButtonProps } from '@mui/material/Button';

import { useBoolean } from 'minimal-shared/hooks';

import Button from '@mui/material/Button';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useActiveWallet } from 'src/actions/account-wallet';
import { deleteFailedPayments, deleteExpiredInvoices } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

interface Props {
  onSuccess: VoidFunction;
  buttonProps?: ButtonProps;
  children?: React.ReactNode;
  transactionType?: TransactionType;
}

export function CleanTransactionsButton({
  onSuccess,
  buttonProps,
  transactionType,
  children,
}: Props) {
  const { t } = useTranslate();
  const isDeleting = useBoolean();
  const { wallet } = useActiveWallet();

  const handleCleanTransactions = async () => {
    if (!wallet?.id) return;

    try {
      isDeleting.onTrue();

      let nInvoicesDeleted = 0;
      let nPaymentsDeleted = 0;

      if (transactionType === TransactionType.INVOICE || !transactionType) {
        const { data } = await deleteExpiredInvoices({ path: { wallet_id: wallet.id } });
        nInvoicesDeleted = data!;
      }

      if (transactionType === TransactionType.PAYMENT || !transactionType) {
        const { data } = await deleteFailedPayments({ path: { wallet_id: wallet.id } });
        nPaymentsDeleted = data!;
      }

      if (nInvoicesDeleted > 0) {
        toast.success(
          t('clean_transactions_button.invoices_deleted_success', { count: nInvoicesDeleted })
        );
        onSuccess();
      }

      if (nPaymentsDeleted > 0) {
        toast.success(
          t('clean_transactions_button.payments_deleted_success', { count: nPaymentsDeleted })
        );
        onSuccess();
      }
    } catch (error) {
      handleActionError(error);
    } finally {
      isDeleting.onFalse();
    }
  };

  return (
    <Button loading={isDeleting.value} onClick={handleCleanTransactions} {...buttonProps}>
      {children || t('clean_transactions_button.clean_transactions')}
    </Button>
  );
}
