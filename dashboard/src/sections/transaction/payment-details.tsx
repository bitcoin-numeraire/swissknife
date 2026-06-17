import type { Payment } from 'src/lib/swissknife';

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
  payment: Payment;
  isAdmin?: boolean;
};

export function PaymentDetails({ payment, isAdmin }: Props) {
  const { t } = useTranslate();
  const { user } = useAuthContext();

  const { ln_address, success_action, payment_hash, payment_preimage } = payment.lightning ?? {};

  const renderList = (
    <Grid container spacing={3} sx={{ my: 5 }}>
      <Grid size={{ xs: 12, md: 4, sm: 6 }}>
        <Title>{t('payment_details.amount_sent')}</Title>
        <SatsWithIcon
          amountMSats={payment.amount_msat - (payment.fee_msat || 0)}
          color="text.secondary"
        />
      </Grid>
      <Grid size={{ xs: 12, md: 4, sm: 6 }}>
        <Title>{t('payment_details.fees')}</Title>
        <SatsWithIcon amountMSats={payment.fee_msat || 0} color="text.secondary" />
      </Grid>
      <Grid size={{ xs: 12, md: 4, sm: 6 }}>
        <Title>{t('payment_details.total_amount')}</Title>
        <SatsWithIcon amountMSats={payment.amount_msat} color="text.secondary" />
      </Grid>

      <Grid size={{ xs: 12 }}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid size={{ xs: 12, sm: 6 }}>
        <Title>{t('transaction_details.description')}</Title>
        <Typography color="textSecondary">{payment.description}</Typography>
      </Grid>
      <Grid size={{ xs: 12, md: 4, sm: 6 }}>
        <Title>{t('payment_details.lightning_address')}</Title>
        <Typography color="textSecondary">{ln_address || 'N/A'}</Typography>
      </Grid>

      <Grid size={{ xs: 12 }}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid size={{ xs: 12, sm: 6 }}>
        <Title>{t('transaction_details.creation_date')}</Title>
        <Typography color="textSecondary">
          {fDate(payment.created_at)} {fTime(payment.created_at)}
        </Typography>
      </Grid>
      <Grid size={{ xs: 12, sm: 6 }}>
        <Title>{t('transaction_details.settlement_date')}</Title>
        <Typography color="textSecondary">
          {fDate(payment.payment_time)} {fTime(payment.payment_time)}
        </Typography>
      </Grid>

      {success_action && (
        <>
          <Grid size={{ xs: 12 }}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid size={{ xs: 12 }}>
            <Title>{t('payment_details.success_action')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              <li>
                {t('payment_details.message')}: {success_action.message}
              </li>
              <li>
                {t('payment_details.success_description')}: {success_action.description}
              </li>
              {success_action.url && (
                <li>
                  URL:{' '}
                  <Link href={success_action.url} target="_blank" rel="noopener noreferrer">
                    {success_action.url}
                  </Link>
                </li>
              )}
            </Typography>
          </Grid>
        </>
      )}

      {payment_hash && (
        <>
          <Grid size={{ xs: 12 }}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid size={{ xs: 12 }}>
            <Title>{t('transaction_details.payment_hash')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {payment_hash}
            </Typography>
          </Grid>
        </>
      )}

      {payment_preimage && (
        <>
          <Grid size={{ xs: 12 }}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid size={{ xs: 12 }}>
            <Title>{t('payment_details.payment_preimage')}</Title>
            <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
              {payment_preimage}
            </Typography>
          </Grid>
        </>
      )}

      {payment.error && (
        <>
          <Grid size={{ xs: 12 }}>
            <Divider sx={{ borderStyle: 'dashed' }} />
          </Grid>

          <Grid size={{ xs: 12 }}>
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
      <TransactionToolbar
        transaction={payment}
        transactionType={TransactionType.PAYMENT}
        isAdmin={isAdmin}
      />
      <Card
        sx={{ pt: 5, px: { xs: 2, sm: 5, md: 8 }, maxWidth: { xs: '100%', md: '80%' }, mx: 'auto' }}
      >
        <Box
          sx={{
            rowGap: 5,
            display: 'grid',
            alignItems: 'center',
            gridTemplateColumns: {
              xs: 'repeat(1, 1fr)',
              sm: 'repeat(2, 1fr)',
            },
          }}
        >
          <Typography variant="subtitle2">{payment.id.toUpperCase()}</Typography>

          <Stack spacing={1} sx={{ alignItems: { xs: 'flex-start', md: 'flex-end' } }}>
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              <Label
                variant="soft"
                color={
                  (payment.status === 'Settled' && 'success') ||
                  (payment.status === 'Pending' && 'warning') ||
                  (payment.status === 'Failed' && 'error') ||
                  'default'
                }
                sx={{ mr: 1 }}
              >
                {payment.status}
              </Label>

              <Label
                variant="soft"
                color={
                  (payment.ledger === 'Lightning' && 'secondary') ||
                  (payment.ledger === 'Internal' && 'primary') ||
                  'default'
                }
              >
                {payment.ledger}
              </Label>
            </Box>
          </Stack>

          <Stack sx={{ typography: 'body2' }}>
            <Typography sx={{ fontWeight: 'bold', mb: 1 }}>
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

const Title = ({ children }: TitleProps) => (
  <Typography variant="subtitle1" gutterBottom sx={{ fontWeight: 'bold' }}>
    {children}
  </Typography>
);
