import type { Invoice, Payment } from 'src/lib/swissknife';
import type { ITransaction } from 'src/types/transaction';

import { useCallback } from 'react';

import Stack from '@mui/material/Stack';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';
import { txidFromOutpoint, bitcoinTransactionExplorerUrl } from 'src/utils/bitcoin-explorer';

import { useTranslate } from 'src/locales';
import { deleteInvoice, deletePayment } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { DeleteButton } from 'src/components/delete';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

type Props = {
  transaction: ITransaction;
  transactionType: TransactionType;
  isAdmin?: boolean;
};

export function TransactionToolbar({ transaction, transactionType, isAdmin }: Props) {
  const { t } = useTranslate();
  const router = useRouter();
  const explorerUrl =
    transactionType === TransactionType.PAYMENT
      ? bitcoinTransactionExplorerUrl((transaction as Payment).bitcoin?.txid)
      : bitcoinTransactionExplorerUrl(
          txidFromOutpoint((transaction as Invoice).bitcoin_output?.outpoint)
        );

  const onDelete = useCallback(
    async (id: string) => {
      try {
        if (transactionType === TransactionType.INVOICE) {
          await deleteInvoice({ path: { id } });

          toast.success(t('transaction_toolbar.delete_invoice_success', { id }));
          router.push(paths.activityList('invoice', isAdmin ? 'admin' : 'wallet'));
        } else {
          await deletePayment({ path: { id } });

          toast.success(t('transaction_toolbar.delete_payment_success', { id }));
          router.push(paths.activityList('payment', isAdmin ? 'admin' : 'wallet'));
        }
      } catch (error) {
        handleActionError(error);
      }
    },
    [isAdmin, router, transactionType, t]
  );

  return (
    <Stack
      spacing={3}
      direction={{ xs: 'column', sm: 'row' }}
      sx={{ mb: { xs: 3, md: 5 }, alignItems: { xs: 'flex-end', sm: 'center' } }}
    >
      <Stack direction="row" spacing={1} sx={{ width: 1, flexGrow: 1 }}>
        {transactionType === TransactionType.INVOICE && (
          <CopyButton
            value={(transaction as Invoice).ln_invoice?.bolt11 || transaction.id}
            title={t('transaction_toolbar.copy_details')}
          />
        )}

        {explorerUrl ? (
          <Tooltip title={t('transaction_actions.open_explorer')}>
            <IconButton component="a" href={explorerUrl} target="_blank" rel="noopener noreferrer">
              <Iconify icon="solar:map-arrow-right-bold" />
            </IconButton>
          </Tooltip>
        ) : (
          <Tooltip title={t('send')}>
            <IconButton
              onClick={() => {
                toast.info(t('coming_soon'));
              }}
            >
              <Iconify icon="iconamoon:send-fill" />
            </IconButton>
          </Tooltip>
        )}

        <Tooltip title={t('share')}>
          <IconButton
            onClick={() => {
              toast.info(t('coming_soon'));
            }}
          >
            <Iconify icon="solar:share-bold" />
          </IconButton>
        </Tooltip>

        {isAdmin && <DeleteButton id={transaction.id} onDelete={onDelete} />}
      </Stack>
    </Stack>
  );
}
