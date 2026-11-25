import type { PaymentSubmission } from 'src/lib/swissknife';

import { useState } from 'react';

import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { cancelWithdrawal, cancelWalletWithdrawal } from 'src/lib/swissknife';

import { SatsWithIcon } from 'src/components/bitcoin';

const labels = {
  accepted: 'send_money.accepted',
  processing: 'send_money.processing',
  waiting: 'send_money.waiting_for_funding',
  broadcast: 'send_money.broadcast',
  reconciling: 'send_money.reconciling',
  cancelled: 'send_money.withdrawal_cancelled',
  expired: 'send_money.withdrawal_expired',
  settled: 'transaction_details.settled',
  failed: 'transaction_details.failed',
} as const;

export function WithdrawalStatus({
  submission,
  isAdmin,
  refresh,
}: {
  submission: PaymentSubmission;
  isAdmin: boolean;
  refresh: () => Promise<unknown>;
}) {
  const { t } = useTranslate();
  const [busy, setBusy] = useState(false);
  const { withdrawal, payment, state } = submission;
  if (!withdrawal) return null;

  const cancel = async () => {
    setBusy(true);
    try {
      const result = isAdmin
        ? await cancelWithdrawal({ path: { id: payment.id } })
        : await cancelWalletWithdrawal({ path: { wallet_id: payment.wallet_id, id: payment.id } });
      if (result.error && result.response?.status !== 409) handleActionError(result.error);
      await refresh();
    } catch (error) {
      handleActionError(error);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Stack spacing={1}>
      <Typography variant="subtitle2">{t(labels[state])}</Typography>
      <Stack
        direction="row"
        spacing={1}
        sx={{ alignItems: 'center', justifyContent: 'space-between' }}
      >
        <Typography variant="body2" color="text.secondary">
          {t('send_money.approved_fee_limit')}
        </Typography>
        <SatsWithIcon amountMSats={withdrawal.maximum_fee_msat} variant="body2" />
      </Stack>
      {withdrawal.awaiting_original_provider && (
        <Alert severity="warning" variant="outlined">
          {t('send_money.original_node')}
        </Alert>
      )}
      {state === 'accepted' && (
        <Button variant="outlined" color="inherit" size="small" loading={busy} onClick={cancel}>
          {t('send_money.cancel_withdrawal')}
        </Button>
      )}
    </Stack>
  );
}
