import type { Payment } from 'src/lib/swissknife';

import Box from '@mui/material/Box';
import Link from '@mui/material/Link';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Typography from '@mui/material/Typography';

import { fSats } from 'src/utils/format-number';
import { getLedgerLabel } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';

import { TransactionType } from 'src/types/transaction';

import { TransactionToolbar } from './transaction-toolbar';
import {
  DetailRow,
  DetailCard,
  MetricTile,
  StatusBadges,
  formatDateTime,
  TransactionTimeline,
} from './transaction-detail-common';

// ----------------------------------------------------------------------

type Props = {
  payment: Payment;
  isAdmin?: boolean;
};

export function PaymentDetails({ payment, isAdmin }: Props) {
  const { t } = useTranslate();
  const { ln_address, success_action, payment_hash, payment_preimage } = payment.lightning ?? {};
  const amountSent = payment.amount_msat || 0;
  const feeAmount = payment.fee_msat || 0;
  const totalAmount = amountSent + feeAmount;
  const feeHelper =
    feeAmount % 1000 !== 0
      ? t('payment_details.exact_msats', { amount: fSats(feeAmount) })
      : undefined;
  const methodLabel = getLedgerLabel(payment.ledger, t);
  const destination =
    ln_address ||
    payment.internal?.ln_address ||
    payment.bitcoin?.address ||
    payment.internal?.btc_address ||
    payment.bitcoin?.txid ||
    payment_hash ||
    methodLabel;
  const settlementTitle =
    (payment.status === 'Failed' && t('transaction_details.failed')) ||
    (payment.status === 'Settled' && t('payment_details.settlement_complete')) ||
    t('payment_details.awaiting_settlement');
  const settlementDescription =
    (payment.status === 'Failed' && (payment.error || t('payment_details.failed_description'))) ||
    (payment.ledger === 'Lightning' && t('payment_details.lightning_settlement_description')) ||
    (payment.ledger === 'Internal' && t('payment_details.internal_settlement_description')) ||
    t('payment_details.onchain_settlement_description');

  return (
    <>
      <TransactionToolbar
        transaction={payment}
        transactionType={TransactionType.PAYMENT}
        isAdmin={isAdmin}
      />

      <Stack spacing={3}>
        <Card sx={{ p: { xs: 3, md: 4 }, borderRadius: 1 }}>
          <Grid container spacing={4} sx={{ alignItems: 'stretch' }}>
            <Grid size={{ xs: 12, md: 7 }}>
              <Stack spacing={3} sx={{ height: 1 }}>
                <Stack spacing={1.25}>
                  <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                    <Box
                      sx={{
                        width: 52,
                        height: 52,
                        display: 'grid',
                        borderRadius: 1,
                        placeItems: 'center',
                        color: 'warning.main',
                        bgcolor: 'warning.lighter',
                      }}
                    >
                      <Iconify icon="eva:diagonal-arrow-right-up-fill" width={30} />
                    </Box>
                    <StatusBadges status={payment.status} ledger={payment.ledger} />
                  </Stack>

                  <Stack spacing={0.5}>
                    <Typography variant="overline" color="text.secondary">
                      {t('payment_details.payment_to')}
                    </Typography>
                    <Typography variant="h4" sx={{ wordBreak: 'break-word' }}>
                      {destination}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {payment.description ||
                        (isAdmin
                          ? t('transaction_details.wallet_account')
                          : t('transaction_details.outgoing_payment'))}
                    </Typography>
                  </Stack>
                </Stack>

                <Stack spacing={0.5}>
                  <Typography variant="caption" color="text.secondary">
                    {t('payment_details.amount_sent')}
                  </Typography>
                  <SatsWithIcon amountMSats={amountSent} variant="h3" />
                </Stack>

                <Grid container spacing={2}>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile
                      title={t('payment_details.amount_sent')}
                      amountMSats={amountSent}
                      helper={t('payment_details.recipient_receives')}
                    />
                  </Grid>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile
                      title={t('payment_details.fees')}
                      amountMSats={feeAmount}
                      helper={feeHelper}
                    />
                  </Grid>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile
                      title={t('payment_details.total_spent')}
                      amountMSats={totalAmount}
                      helper={t('payment_details.amount_plus_fees')}
                    />
                  </Grid>
                </Grid>
              </Stack>
            </Grid>

            <Grid size={{ xs: 12, md: 5 }}>
              <Card
                variant="outlined"
                sx={{
                  p: 3,
                  height: 1,
                  borderRadius: 1,
                  bgcolor: 'background.neutral',
                }}
              >
                <Stack spacing={2}>
                  <Stack spacing={0.5}>
                    <Typography variant="overline" color="text.secondary">
                      {t('payment_details.settlement')}
                    </Typography>
                    <Typography variant="h6">{settlementTitle}</Typography>
                    <Typography variant="body2" color="text.secondary">
                      {settlementDescription}
                    </Typography>
                  </Stack>

                  <Stack spacing={1}>
                    <Box
                      sx={[
                        (theme) => ({
                          p: 2,
                          borderRadius: 1,
                          bgcolor: 'background.paper',
                          border: `1px solid ${theme.vars.palette.divider}`,
                        }),
                      ]}
                    >
                      <Stack spacing={0.75}>
                        <Typography variant="caption" color="text.secondary">
                          {t('payment_details.recipient')}
                        </Typography>
                        <Typography variant="subtitle2" sx={{ wordBreak: 'break-word' }}>
                          {destination}
                        </Typography>
                        <Typography variant="caption" color="text.disabled">
                          {t('payment_details.destination')}
                        </Typography>
                      </Stack>
                    </Box>

                    <Box
                      sx={[
                        (theme) => ({
                          p: 2,
                          borderRadius: 1,
                          bgcolor: 'background.paper',
                          border: `1px solid ${theme.vars.palette.divider}`,
                        }),
                      ]}
                    >
                      <Stack
                        direction="row"
                        spacing={1}
                        sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                      >
                        <Stack spacing={0.5}>
                          <Typography variant="caption" color="text.secondary">
                            {t('payment_details.method')}
                          </Typography>
                          <Typography variant="subtitle2">{methodLabel}</Typography>
                        </Stack>
                        <Stack spacing={0.5} sx={{ alignItems: 'flex-end' }}>
                          <Typography variant="caption" color="text.secondary">
                            {t('payment_details.total_spent')}
                          </Typography>
                          <SatsWithIcon amountMSats={totalAmount} variant="subtitle2" />
                        </Stack>
                      </Stack>
                    </Box>
                  </Stack>

                  {payment.error && (
                    <Alert severity="error" variant="outlined">
                      {payment.error}
                    </Alert>
                  )}

                  {success_action && (
                    <Alert severity="success" variant="outlined">
                      <Stack spacing={0.5}>
                        <Typography variant="subtitle2">
                          {success_action.description || t('payment_details.success_action')}
                        </Typography>
                        {success_action.message && (
                          <Typography variant="body2">{success_action.message}</Typography>
                        )}
                        {success_action.url && (
                          <Link href={success_action.url} target="_blank" rel="noopener noreferrer">
                            {success_action.url}
                          </Link>
                        )}
                      </Stack>
                    </Alert>
                  )}
                </Stack>
              </Card>
            </Grid>
          </Grid>
        </Card>

        <Grid container spacing={3}>
          <Grid size={{ xs: 12, md: 5 }}>
            <DetailCard
              title={t('transaction_details.timeline')}
              icon="solar:sort-by-time-bold-duotone"
            >
              <TransactionTimeline
                items={[
                  {
                    label: t('transaction_details.creation_date'),
                    value: payment.created_at,
                    state: 'done',
                  },
                  {
                    label: t('transaction_details.settlement_date'),
                    value: payment.payment_time,
                    state:
                      (payment.status === 'Settled' && 'done') ||
                      (payment.status === 'Failed' && 'error') ||
                      'waiting',
                  },
                ]}
              />
            </DetailCard>
          </Grid>

          <Grid size={{ xs: 12, md: 7 }}>
            <DetailCard
              title={t('transaction_details.payment_context')}
              icon="solar:document-text-bold-duotone"
            >
              <DetailRow label={t('transaction_details.description')} value={payment.description} />
              <DetailRow label={t('payment_details.lightning_address')} value={ln_address} />
              <DetailRow
                label={t('payment_details.onchain_address')}
                value={payment.bitcoin?.address}
              />
              <DetailRow
                label={t('payment_details.internal_address')}
                value={payment.internal?.ln_address}
              />
              <DetailRow label={t('transaction_details.ledger')} value={methodLabel} />
              <DetailRow label={t('transaction_details.currency')} value={payment.currency} />
              <DetailRow
                label={t('transaction_details.created')}
                value={formatDateTime(payment.created_at)}
              />
              <DetailRow
                label={t('transaction_details.settled_at')}
                value={formatDateTime(payment.payment_time)}
              />
            </DetailCard>
          </Grid>

          <Grid size={{ xs: 12 }}>
            <DetailCard
              title={t('transaction_details.technical_details')}
              icon="solar:code-square-bold-duotone"
            >
              <DetailRow
                label={t('transaction_details.transaction_id')}
                value={payment.id}
                copyValue={payment.id}
                mono
              />
              <DetailRow
                label={t('transaction_details.wallet_id')}
                value={payment.wallet_id}
                copyValue={payment.wallet_id}
                mono
              />
              <DetailRow
                label={t('transaction_details.payment_hash')}
                value={payment_hash}
                copyValue={payment_hash ?? undefined}
                mono
              />
              <DetailRow
                label={t('payment_details.payment_preimage')}
                value={payment_preimage}
                copyValue={payment_preimage ?? undefined}
                mono
              />
              <DetailRow
                label={t('payment_details.onchain_txid')}
                value={payment.bitcoin?.txid}
                copyValue={payment.bitcoin?.txid ?? undefined}
                mono
              />
              <DetailRow
                label={t('payment_details.onchain_address')}
                value={payment.bitcoin?.address || payment.internal?.btc_address}
                copyValue={payment.bitcoin?.address || payment.internal?.btc_address || undefined}
                mono
              />
              <DetailRow
                label={t('payment_details.error_message')}
                value={payment.error}
                copyValue={payment.error ?? undefined}
                mono
              />
            </DetailCard>
          </Grid>
        </Grid>
      </Stack>
    </>
  );
}
