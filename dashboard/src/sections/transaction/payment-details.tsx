import type { Payment } from 'src/lib/swissknife';

import Box from '@mui/material/Box';
import Link from '@mui/material/Link';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Typography from '@mui/material/Typography';

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
  const amountWithoutFees = payment.amount_msat - (payment.fee_msat || 0);

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
                      {t('payment_details.payment_from')}
                    </Typography>
                    <Typography variant="h4">
                      {payment.description || ln_address || t('recent_transactions.empty_description')}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {isAdmin
                        ? t('transaction_details.wallet_account')
                        : t('transaction_details.outgoing_payment')}
                    </Typography>
                  </Stack>
                </Stack>

                <Stack spacing={0.5}>
                  <Typography variant="caption" color="text.secondary">
                    {t('payment_details.total_amount')}
                  </Typography>
                  <SatsWithIcon amountMSats={payment.amount_msat} variant="h3" />
                </Stack>

                <Grid container spacing={2}>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile
                      title={t('payment_details.amount_sent')}
                      amountMSats={amountWithoutFees}
                      helper={payment.status === 'Settled' ? t('transaction_details.settled') : payment.status}
                    />
                  </Grid>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile title={t('payment_details.fees')} amountMSats={payment.fee_msat || 0} />
                  </Grid>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <Box
                      sx={[
                        (theme) => ({
                          p: 2,
                          height: 1,
                          borderRadius: 1,
                          bgcolor: 'background.neutral',
                          border: `1px solid ${theme.vars.palette.divider}`,
                        }),
                      ]}
                    >
                      <Stack spacing={0.75}>
                        <Typography variant="caption" color="text.secondary">
                          {t('payment_details.recipient')}
                        </Typography>
                        <Typography variant="subtitle1" noWrap>
                          {ln_address || payment.ledger}
                        </Typography>
                        <Typography variant="caption" color="text.disabled">
                          {t('payment_details.destination')}
                        </Typography>
                      </Stack>
                    </Box>
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
                      {t('payment_details.delivery')}
                    </Typography>
                    <Typography variant="h6">
                      {payment.status === 'Settled'
                        ? t('payment_details.delivered')
                        : t('payment_details.awaiting_settlement')}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {payment.status === 'Failed'
                        ? payment.error || t('payment_details.failed_description')
                        : t('payment_details.delivery_description')}
                    </Typography>
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
            <DetailCard title={t('transaction_details.timeline')} icon="solar:sort-by-time-bold-duotone">
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
            <DetailCard title={t('transaction_details.payment_context')} icon="solar:document-text-bold-duotone">
              <DetailRow label={t('transaction_details.description')} value={payment.description} />
              <DetailRow label={t('payment_details.lightning_address')} value={ln_address} />
              <DetailRow label={t('transaction_details.ledger')} value={payment.ledger} />
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
            <DetailCard title={t('transaction_details.technical_details')} icon="solar:code-square-bold-duotone">
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
