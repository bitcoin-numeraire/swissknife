'use client';

import type { ITransaction } from 'src/types/transaction';

import { useState } from 'react';

import Button from '@mui/material/Button';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { deleteInvoices, deletePayments } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { ConfirmDialog } from 'src/components/custom-dialog';

import { TransactionType } from 'src/types/transaction';

type Props = {
  transactions: ITransaction[];
  onDeleted: (transactions: ITransaction[]) => void;
};

export function DeleteSelectedTransactionsButton({ transactions, onDeleted }: Props) {
  const { t } = useTranslate();
  const [pending, setPending] = useState<ITransaction[] | null>(null);
  const [deleting, setDeleting] = useState(false);

  const handleDelete = async () => {
    if (!pending?.length || deleting) return;

    setDeleting(true);
    const completed: ITransaction[] = [];
    let count = 0;

    try {
      // Each endpoint accepts IDs of its own resource type. Never send an empty
      // selector or broaden the selection to a status or wallet filter.
      for (const type of [TransactionType.INVOICE, TransactionType.PAYMENT]) {
        const group = pending.filter((transaction) => transaction.transaction_type === type);
        if (!group.length) continue;

        const options = {
          query: { ids: group.map((transaction) => transaction.id) },
          throwOnError: true as const,
        };
        const { data } =
          type === TransactionType.INVOICE
            ? await deleteInvoices<true>(options)
            : await deletePayments<true>(options);
        completed.push(...group);
        count += data;
      }
    } catch (error) {
      handleActionError(error);
    } finally {
      // These requests are not atomic across resource types. Refresh successful
      // groups and keep failed groups selected so they can be retried.
      if (completed.length) {
        toast.success(t('transaction_list.transactions_deleted', { count }));
        onDeleted(completed);
      }
      setDeleting(false);
      setPending(null);
    }
  };

  return (
    <>
      <Tooltip title={t('activity_view.delete_selected')}>
        <IconButton
          color="error"
          aria-label={t('activity_view.delete_selected')}
          disabled={!transactions.length || deleting}
          onClick={() => setPending([...transactions])}
        >
          <Iconify icon="solar:trash-bin-trash-bold" />
        </IconButton>
      </Tooltip>
      <ConfirmDialog
        open={pending !== null}
        title={t('activity_view.delete_selected')}
        content={t('confirm_delete_content', { count: pending?.length ?? 0 })}
        onClose={() => {
          if (!deleting) setPending(null);
        }}
        action={
          <Button color="error" variant="contained" loading={deleting} onClick={handleDelete}>
            {t('delete')}
          </Button>
        }
      />
    </>
  );
}
