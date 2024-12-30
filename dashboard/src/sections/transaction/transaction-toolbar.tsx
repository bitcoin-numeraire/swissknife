import type { InvoiceResponse } from 'src/lib/swissknife';
import type { ITransaction } from 'src/types/transaction';

import { useCallback } from 'react';

import Stack from '@mui/material/Stack';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

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

  const onDelete = useCallback(
    async (id: string) => {
      try {
        if (transactionType === TransactionType.INVOICE) {
          await deleteInvoice({ path: { id } });

          toast.success(t('transaction_toolbar.delete_invoice_success', { id }));
          router.push(paths.admin.invoices);
        } else {
          await deletePayment({ path: { id } });

          toast.success(t('transaction_toolbar.delete_payment_success', { id }));
          router.push(paths.admin.payments);
        }
      } catch (error) {
        handleActionError(error);
      }
    },
    [router, transactionType, t]
  );

  return (
    <Stack
      spacing={3}
      direction={{ xs: 'column', sm: 'row' }}
      alignItems={{ xs: 'flex-end', sm: 'center' }}
      sx={{ mb: { xs: 3, md: 5 } }}
    >
      <Stack direction="row" spacing={1} flexGrow={1} sx={{ width: 1 }}>
        {transactionType === TransactionType.INVOICE && (
          <CopyButton
            value={(transaction as InvoiceResponse).ln_invoice?.bolt11 || transaction.id}
            title={t('transaction_toolbar.copy_details')}
          />
        )}

        <Tooltip title={t('send')}>
          <IconButton
            onClick={() => {
              toast.info(t('coming_soon'));
            }}
          >
            <Iconify icon="iconamoon:send-fill" />
          </IconButton>
        </Tooltip>

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
