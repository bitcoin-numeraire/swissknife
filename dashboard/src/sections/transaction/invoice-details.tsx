import type { InvoiceResponse } from 'src/lib/swissknife';

import { QRCode } from 'react-qrcode-logo';

import Box from '@mui/material/Box';
import { Grid } from '@mui/material';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';

import { fDate, fTime } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';
import { SatsWithIcon } from 'src/components/bitcoin';

import { useAuthContext } from 'src/auth/hooks';

import { TransactionType } from 'src/types/transaction';

import { TransactionToolbar } from './transaction-toolbar';

// ----------------------------------------------------------------------

type Props = {
  invoice: InvoiceResponse;
  isAdmin?: boolean;
};

export function InvoiceDetails({ invoice, isAdmin }: Props) {
  const { t } = useTranslate();
  const { user } = useAuthContext();

  const renderList = (
    <Grid container spacing={3} sx={{ my: 5 }}>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('invoice_details.amount_requested')}</Title>
        <SatsWithIcon amountMSats={invoice.amount_msat || 0} color="text.secondary" />
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('invoice_details.amount_received')}</Title>
        {invoice.status === 'Settled' && (
          <SatsWithIcon amountMSats={(invoice.amount_received_msat || 0) - (invoice.fee_msat || 0)} color="text.secondary" />
        )}
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('invoice_details.fees')}</Title>
        <SatsWithIcon amountMSats={invoice.fee_msat || 0} color="text.secondary" />
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12}>
        <Title>{t('transaction_details.description')}</Title>
        <Typography color="textSecondary">{invoice.description}</Typography>
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('transaction_details.creation_date')}</Title>
        <Typography color="textSecondary">
          {fDate(invoice.timestamp)} {fTime(invoice.timestamp)}
        </Typography>
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('invoice_details.expiration_date')}</Title>
        <Typography color="textSecondary">
          {fDate(invoice.ln_invoice?.expires_at)} {fTime(invoice.ln_invoice?.expires_at)}
        </Typography>
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('transaction_details.settlement_date')}</Title>
        <Typography color="textSecondary">
          {fDate(invoice.payment_time)} {fTime(invoice.payment_time)}
        </Typography>
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('transaction_details.ledger')}</Title>
        <Typography color="textSecondary">{invoice.ledger}</Typography>
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('transaction_details.currency')}</Title>
        <Typography color="textSecondary">{invoice.currency}</Typography>
      </Grid>
      {invoice.ln_invoice && (
        <Grid item xs={12} md={4} sm={6}>
          <Title>{t('invoice_details.min_final_cltv_delta')}</Title>
          <Typography color="textSecondary">{invoice.ln_invoice?.min_final_cltv_expiry_delta}</Typography>
        </Grid>
      )}

      {invoice.ln_invoice && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>Bolt11</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {invoice.ln_invoice?.bolt11}
            </Typography>
          </Grid>
        </>
      )}

      {invoice.ln_invoice && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('transaction_details.payment_hash')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {invoice.ln_invoice?.payment_hash}
            </Typography>
          </Grid>
        </>
      )}

      {invoice.ln_invoice && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('invoice_details.payment_secret')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {invoice.ln_invoice?.payment_secret}
            </Typography>
          </Grid>
        </>
      )}

      {invoice.ln_invoice && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ my: 1, borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('invoice_details.payee_pubkey')}</Title>
            <Typography color="textSecondary">{invoice.ln_invoice?.payee_pubkey}</Typography>
          </Grid>
        </>
      )}
    </Grid>
  );

  return (
    <>
      <TransactionToolbar transaction={invoice} transactionType={TransactionType.INVOICE} isAdmin={isAdmin} />
      <Card sx={{ pt: 5, px: { xs: 2, sm: 5, md: 8 }, maxWidth: { xs: '100%', md: '80%' }, mx: 'auto' }}>
        <Box
          rowGap={5}
          display="grid"
          alignItems="center"
          gridTemplateColumns={{
            xs: 'repeat(1, 1fr)',
            sm: 'repeat(2, 1fr)',
          }}
        >
          <Typography variant="subtitle2">{invoice.id.toUpperCase()}</Typography>

          <Stack spacing={1} alignItems={{ xs: 'flex-start', md: 'flex-end' }}>
            <Box display="flex" alignItems="center">
              <Label
                variant="soft"
                color={
                  (invoice.status === 'Settled' && 'success') ||
                  (invoice.status === 'Pending' && 'warning') ||
                  (invoice.status === 'Expired' && 'error') ||
                  'default'
                }
                mr={1}
              >
                {invoice.status}
              </Label>

              <Label
                variant="soft"
                color={(invoice.ledger === 'Lightning' && 'secondary') || (invoice.ledger === 'Internal' && 'primary') || 'default'}
              >
                {invoice.ledger}
              </Label>
            </Box>
          </Stack>

          <Stack sx={{ typography: 'body2' }}>
            <Typography fontWeight="bold" sx={{ mb: 1 }}>
              {t('invoice_details.invoice_from')}
            </Typography>
            {isAdmin ? (
              invoice.wallet_id
            ) : (
              <>
                {user?.displayName}
                <br />
                {user?.email}
                <br />
              </>
            )}
          </Stack>

          {invoice.ln_invoice && (
            <Stack sx={{ typography: 'body2' }}>
              <Box
                sx={{
                  width: '100%',
                  maxWidth: 300,
                  height: 'auto',
                  '& > canvas': {
                    width: '100% !important',
                    height: 'auto !important',
                  },
                }}
              >
                <QRCode
                  value={invoice.ln_invoice.bolt11}
                  size={300} // Base size, will be overridden by CSS
                  logoImage="/logo/logo_square_negative.svg"
                  removeQrCodeBehindLogo
                  logoPaddingStyle="circle"
                  eyeRadius={5}
                  logoPadding={3}
                />
              </Box>
            </Stack>
          )}
        </Box>

        {renderList}
      </Card>
    </>
  );
}

type TitleProps = {
  children: React.ReactNode;
};

const Title = ({ children }: TitleProps): JSX.Element => (
  <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
    {children}
  </Typography>
);
