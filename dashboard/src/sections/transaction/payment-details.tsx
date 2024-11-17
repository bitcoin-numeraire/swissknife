import type { PaymentResponse } from 'src/lib/swissknife';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import { Grid, Link } from '@mui/material';
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
  payment: PaymentResponse;
  isAdmin?: boolean;
};

export function PaymentDetails({ payment, isAdmin }: Props) {
  const { t } = useTranslate();
  const { user } = useAuthContext();

  const renderList = (
    <Grid container spacing={3} sx={{ my: 5 }}>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('payment_details.amount_sent')}</Title>
        <SatsWithIcon amountMSats={payment.amount_msat - (payment.fee_msat || 0)} color="text.secondary" />
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('payment_details.fees')}</Title>
        <SatsWithIcon amountMSats={payment.fee_msat || 0} color="text.secondary" />
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('payment_details.total_amount')}</Title>
        <SatsWithIcon amountMSats={payment.amount_msat} color="text.secondary" />
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12} sm={6}>
        <Title>{t('transaction_details.description')}</Title>
        <Typography color="textSecondary">{payment.description}</Typography>
      </Grid>
      <Grid item xs={12} md={4} sm={6}>
        <Title>{t('payment_details.lightning_address')}</Title>
        <Typography color="textSecondary">{payment.ln_address || 'N/A'}</Typography>
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12} sm={6}>
        <Title>{t('transaction_details.creation_date')}</Title>
        <Typography color="textSecondary">
          {fDate(payment.created_at)} {fTime(payment.created_at)}
        </Typography>
      </Grid>
      <Grid item xs={12} sm={6}>
        <Title>{t('transaction_details.settlement_date')}</Title>
        <Typography color="textSecondary">
          {fDate(payment.payment_time)} {fTime(payment.payment_time)}
        </Typography>
      </Grid>

      {payment.success_action && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('payment_details.success_action')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              <li>
                {t('payment_details.message')}: {payment.success_action.message}
              </li>
              <li>
                {t('payment_details.success_description')}: {payment.success_action.description}
              </li>
              {payment.success_action.url && (
                <li>
                  URL:{' '}
                  <Link href={payment.success_action.url} target="_blank" rel="noopener noreferrer">
                    {payment.success_action.url}
                  </Link>
                </li>
              )}
            </Typography>
          </Grid>
        </>
      )}

      {payment.payment_hash && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('transaction_details.payment_hash')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {payment.payment_hash}
            </Typography>
          </Grid>
        </>
      )}

      {payment.payment_preimage && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('payment_details.payment_preimage')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {payment.payment_preimage}
            </Typography>
          </Grid>
        </>
      )}

      {payment.error && (
        <>
          <Grid item xs={12}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid item xs={12}>
            <Title>{t('payment_details.error_message')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {payment.error}
            </Typography>
          </Grid>
        </>
      )}
    </Grid>
  );

  return (
    <>
      <TransactionToolbar transaction={payment} transactionType={TransactionType.PAYMENT} isAdmin={isAdmin} />
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
          <Typography variant="subtitle2">{payment.id.toUpperCase()}</Typography>

          <Stack spacing={1} alignItems={{ xs: 'flex-start', md: 'flex-end' }}>
            <Box display="flex" alignItems="center">
              <Label
                variant="soft"
                color={
                  (payment.status === 'Settled' && 'success') ||
                  (payment.status === 'Pending' && 'warning') ||
                  (payment.status === 'Failed' && 'error') ||
                  'default'
                }
                mr={1}
              >
                {payment.status}
              </Label>

              <Label
                variant="soft"
                color={(payment.ledger === 'Lightning' && 'secondary') || (payment.ledger === 'Internal' && 'primary') || 'default'}
              >
                {payment.ledger}
              </Label>
            </Box>
          </Stack>

          <Stack sx={{ typography: 'body2' }}>
            <Typography fontWeight="bold" sx={{ mb: 1 }}>
              {t('payment_details.payment_from')}
            </Typography>
            {isAdmin ? (
              payment.wallet_id
            ) : (
              <>
                {user?.displayName}
                <br />
                {user?.email}
                <br />
              </>
            )}
          </Stack>
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
