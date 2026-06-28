import type { Invoice } from 'src/lib/swissknife';

import { QRCode } from 'react-qrcode-logo';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Typography from '@mui/material/Typography';

import { getLedgerLabel } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';

import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';

import { TransactionType } from 'src/types/transaction';

import { TransactionToolbar } from './transaction-toolbar';
import {
  DetailRow,
  DetailCard,
  MetricTile,
  StatusBadges,
  formatDateTime,
  TransactionDirectionIcon,
  TransactionTimeline,
} from './transaction-detail-common';

// ----------------------------------------------------------------------

type Props = {
  invoice: Invoice;
  isAdmin?: boolean;
};

export function InvoiceDetails({ invoice, isAdmin }: Props) {
  const { t } = useTranslate();
  const bolt11 = invoice.ln_invoice?.bolt11;
  const methodLabel = getLedgerLabel(invoice.ledger, t);
  const isOpenAmount = !invoice.amount_msat;
  const receivedAmount =
    invoice.status === 'Settled'
      ? (invoice.amount_received_msat || 0) - (invoice.fee_msat || 0)
      : 0;
  const totalAmount = isOpenAmount
    ? invoice.status === 'Settled'
      ? invoice.amount_received_msat || 0
      : undefined
    : (invoice.amount_msat || 0) + (invoice.fee_msat || 0);

  return (
    <>
      <TransactionToolbar
        transaction={invoice}
        transactionType={TransactionType.INVOICE}
        isAdmin={isAdmin}
      />

      <Stack spacing={3}>
        <Card sx={{ p: { xs: 3, md: 4 }, borderRadius: 1 }}>
          <Grid container spacing={4} sx={{ alignItems: 'stretch' }}>
            <Grid size={{ xs: 12, md: 7 }}>
              <Stack spacing={3} sx={{ height: 1 }}>
                <Stack spacing={1.25}>
                  <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                    <TransactionDirectionIcon direction="in" />
                    <StatusBadges status={invoice.status} ledger={invoice.ledger} />
                  </Stack>

                  <Stack spacing={0.5}>
                    <Typography variant="overline" color="text.secondary">
                      {t('invoice_details.request_memo')}
                    </Typography>
                    <Typography variant="h4">
                      {invoice.description || t('transaction_details.incoming_invoice')}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {isAdmin
                        ? t('transaction_details.wallet_account')
                        : t('transaction_details.incoming_invoice')}
                    </Typography>
                  </Stack>
                </Stack>

                <Stack spacing={0.5}>
                  <Typography variant="caption" color="text.secondary">
                    {t('invoice_details.amount_requested')}
                  </Typography>
                  {isOpenAmount ? (
                    <Typography variant="h3">{t('wallet_view.open_amount')}</Typography>
                  ) : (
                    <SatsWithIcon amountMSats={invoice.amount_msat || 0} variant="h3" />
                  )}
                </Stack>

                <Grid container spacing={{ xs: 1, sm: 2 }}>
                  <Grid size={{ xs: 4 }}>
                    <MetricTile
                      title={t('invoice_details.amount_received')}
                      amountMSats={receivedAmount}
                      helper={
                        invoice.status === 'Settled'
                          ? t('transaction_details.settled')
                          : t('transaction_details.pending')
                      }
                    />
                  </Grid>
                  <Grid size={{ xs: 4 }}>
                    <MetricTile
                      title={t('invoice_details.fees')}
                      amountMSats={invoice.fee_msat || 0}
                    />
                  </Grid>
                  <Grid size={{ xs: 4 }}>
                    <MetricTile
                      title={t('transaction_details.total')}
                      amountMSats={totalAmount}
                      value={totalAmount === undefined ? t('wallet_view.open_amount') : undefined}
                    />
                  </Grid>
                </Grid>
              </Stack>
            </Grid>

            <Grid size={{ xs: 12, md: 5 }}>
              <Card
                variant="outlined"
                sx={{
                  p: 2,
                  height: 1,
                  borderRadius: 1,
                  bgcolor: 'background.neutral',
                }}
              >
                {bolt11 ? (
                  <Stack spacing={2}>
                    <Box
                      sx={{
                        p: 2,
                        borderRadius: 1,
                        bgcolor: 'common.white',
                        '& canvas': { width: '100% !important', height: 'auto !important' },
                      }}
                    >
                      <QRCode
                        value={bolt11}
                        size={320}
                        logoImage="/logo/logo_square_negative.svg"
                        removeQrCodeBehindLogo
                        logoPaddingStyle="circle"
                        eyeRadius={5}
                        logoPadding={3}
                      />
                    </Box>

                    <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center' }}>
                      <Typography variant="caption" color="text.secondary" noWrap sx={{ flex: 1 }}>
                        {bolt11}
                      </Typography>
                      <CopyButton value={bolt11} title={t('invoice_details.copy_bolt11')} />
                    </Stack>
                  </Stack>
                ) : (
                  <Alert severity="info" variant="outlined">
                    {t('invoice_details.no_lightning_payload')}
                  </Alert>
                )}
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
                    value: invoice.timestamp || invoice.created_at,
                    state: 'done',
                  },
                  {
                    label: t('invoice_details.expiration_date'),
                    value: invoice.ln_invoice?.expires_at,
                    state: invoice.status === 'Expired' ? 'error' : 'waiting',
                  },
                  {
                    label: t('transaction_details.settlement_date'),
                    value: invoice.payment_time,
                    state: invoice.status === 'Settled' ? 'done' : 'waiting',
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
              <DetailRow label={t('transaction_details.description')} value={invoice.description} />
              <DetailRow label={t('transaction_details.ledger')} value={methodLabel} />
              <DetailRow label={t('transaction_details.currency')} value={invoice.currency} />
              <DetailRow
                label={t('transaction_details.created')}
                value={formatDateTime(invoice.timestamp || invoice.created_at)}
              />
              <DetailRow
                label={t('transaction_details.settled_at')}
                value={formatDateTime(invoice.payment_time)}
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
                value={invoice.id}
                copyValue={invoice.id}
                mono
              />
              <DetailRow
                label={t('transaction_details.wallet_id')}
                value={invoice.wallet_id}
                copyValue={invoice.wallet_id}
                mono
              />
              {invoice.ln_invoice && (
                <>
                  <DetailRow
                    label={t('invoice_details.bolt11')}
                    value={invoice.ln_invoice.bolt11}
                    copyValue={invoice.ln_invoice.bolt11}
                    mono
                  />
                  <DetailRow
                    label={t('transaction_details.payment_hash')}
                    value={invoice.ln_invoice.payment_hash}
                    copyValue={invoice.ln_invoice.payment_hash}
                    mono
                  />
                  <DetailRow
                    label={t('invoice_details.payment_secret')}
                    value={invoice.ln_invoice.payment_secret}
                    copyValue={invoice.ln_invoice.payment_secret}
                    mono
                  />
                  <DetailRow
                    label={t('invoice_details.payee_pubkey')}
                    value={invoice.ln_invoice.payee_pubkey}
                    copyValue={invoice.ln_invoice.payee_pubkey}
                    mono
                  />
                  <DetailRow
                    label={t('invoice_details.min_final_cltv_delta')}
                    value={invoice.ln_invoice.min_final_cltv_expiry_delta}
                  />
                </>
              )}
            </DetailCard>
          </Grid>
        </Grid>
      </Stack>
    </>
  );
}
