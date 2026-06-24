'use client';

import type { Contact } from 'src/lib/swissknife';
import type { ITransaction } from 'src/types/transaction';

import { mutate } from 'swr';
import { useMemo } from 'react';
import { sumBy } from 'es-toolkit';
import { useBoolean } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { paths } from 'src/routes/paths';

import { satsToFiat } from 'src/utils/fiat';
import { shouldFail } from 'src/utils/errors';
import { fFromNow } from 'src/utils/format-time';
import { fCurrency } from 'src/utils/format-number';
import { mergeAndSortTransactions } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetUserWallet } from 'src/actions/user-wallet';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { useSettingsContext } from 'src/components/settings';

import { TransactionType } from 'src/types/transaction';

import { SendMoneyDrawer, ReceiveMoneyDrawer } from '../money-drawers';

// ----------------------------------------------------------------------

const fallbackFiatPrices = { USD: 0, EUR: 0, CHF: 0 };

function txAmount(tx: ITransaction) {
  return (tx.amount_msat || 0) + (tx.fee_msat || 0);
}

function txDirection(tx: ITransaction) {
  return tx.transaction_type === TransactionType.INVOICE ? 'in' : 'out';
}

function statusColor(status: string) {
  if (status === 'Settled') return 'success';
  if (status === 'Failed' || status === 'Expired') return 'error';
  return 'warning';
}

function railIcon(tx: ITransaction) {
  if (tx.ledger === 'Onchain') return 'solar:link-bold-duotone';
  if (tx.ledger === 'Internal') return 'solar:home-angle-bold-duotone';
  return 'solar:bolt-bold-duotone';
}

// ----------------------------------------------------------------------

export function WalletView() {
  const sendDrawer = useBoolean();
  const receiveDrawer = useBoolean();
  const { t } = useTranslate();
  const { state } = useSettingsContext();

  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const { fiatPrices, fiatPricesError } = useFetchFiatPrices();

  const prices = fiatPrices ?? fallbackFiatPrices;
  const errors = [walletError];
  const data = [wallet];
  const isLoading = [walletLoading];
  const failed = shouldFail(errors, data, isLoading);

  const allTransactions = useMemo(
    () => mergeAndSortTransactions(wallet?.invoices || [], wallet?.payments || []),
    [wallet?.invoices, wallet?.payments]
  );

  const contacts: Contact[] = useMemo(() => wallet?.contacts || [], [wallet?.contacts]);

  const pendingIncoming = useMemo(
    () =>
      wallet?.invoices.filter((invoice) => invoice.status === 'Pending').reduce(
        (total, invoice) => total + (invoice.amount_msat || 0),
        0
      ) ?? 0,
    [wallet?.invoices]
  );

  const failedPayments = useMemo(
    () => wallet?.payments.filter((payment) => payment.status === 'Failed') ?? [],
    [wallet?.payments]
  );

  const settledIn = useMemo(
    () =>
      sumBy(
        wallet?.invoices.filter((invoice) => invoice.status === 'Settled') ?? [],
        (invoice) => invoice.amount_received_msat || invoice.amount_msat || 0
      ),
    [wallet?.invoices]
  );

  const settledOut = useMemo(
    () =>
      sumBy(
        wallet?.payments.filter((payment) => payment.status === 'Settled') ?? [],
        (payment) => payment.amount_msat || 0
      ),
    [wallet?.payments]
  );

  const onMutation = () => mutate(endpointKeys.userWallet.get);

  return (
    <DashboardContent maxWidth="xl">
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <Stack
            direction={{ xs: 'column', md: 'row' }}
            spacing={2}
            sx={{ alignItems: { md: 'flex-end' }, justifyContent: 'space-between', mb: 3 }}
          >
            <Stack spacing={1}>
              <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                <Label color="info">{t('wallet_view.scope_account')}</Label>
                <Label color="default">{t('wallet_view.single_network')}</Label>
              </Stack>
              <Typography variant="h3">{t('wallet_view.title')}</Typography>
              <Typography variant="body2" color="text.secondary">
                {wallet?.id}
              </Typography>
            </Stack>

            <Stack direction="row" spacing={1}>
              <Button
                color="inherit"
                variant="outlined"
                startIcon={<Iconify icon="solar:arrow-up-linear" />}
                onClick={sendDrawer.onTrue}
              >
                {t('send')}
              </Button>
              <Button
                color="inherit"
                variant="contained"
                startIcon={<Iconify icon="solar:arrow-down-linear" />}
                onClick={receiveDrawer.onTrue}
              >
                {t('wallet_view.receive')}
              </Button>
            </Stack>
          </Stack>

          {fiatPricesError && (
            <Alert severity="warning" sx={{ mb: 3 }}>
              {t('wallet_view.fiat_unavailable')}
            </Alert>
          )}

          <Grid container spacing={3}>
            <Grid size={{ xs: 12, lg: 8 }}>
              <Card sx={{ p: 3, borderRadius: 1, minHeight: 352 }}>
                <Stack spacing={3}>
                  <Stack
                    direction={{ xs: 'column', sm: 'row' }}
                    spacing={2}
                    sx={{ justifyContent: 'space-between' }}
                  >
                    <Stack spacing={1}>
                      <Typography variant="overline" color="text.secondary">
                        {t('wallet_view.spendable_now')}
                      </Typography>
                      <SatsWithIcon
                        variant="h2"
                        amountMSats={wallet?.balance.available_msat ?? 0}
                        sx={{ letterSpacing: 0 }}
                      />
                      <Typography variant="subtitle1" color="text.secondary">
                        {fCurrency(
                          satsToFiat(
                            (wallet?.balance.available_msat ?? 0) / 1000,
                            prices,
                            state.currency
                          ),
                          { currency: state.currency }
                        )}
                      </Typography>
                    </Stack>

                    <Box
                      sx={[
                        (theme) => ({
                          p: 2,
                          width: { xs: 1, sm: 260 },
                          borderRadius: 1,
                          bgcolor: 'background.neutral',
                          border: `1px solid ${theme.vars.palette.divider}`,
                        }),
                      ]}
                    >
                      <Stack spacing={1.5}>
                        <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                          <Typography variant="body2" color="text.secondary">
                            {t('wallet_view.arriving_soon')}
                          </Typography>
                          <SatsWithIcon amountMSats={pendingIncoming} />
                        </Stack>
                        <LinearProgress
                          variant="determinate"
                          value={Math.min(
                            100,
                            ((wallet?.balance.available_msat ?? 0) /
                              Math.max(1, (wallet?.balance.received_msat ?? 0) + pendingIncoming)) *
                              100
                          )}
                          color="success"
                        />
                        <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                          <Typography variant="body2" color="text.secondary">
                            {t('wallet_view.sent')}
                          </Typography>
                          <SatsWithIcon amountMSats={settledOut} />
                        </Stack>
                        <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                          <Typography variant="body2" color="text.secondary">
                            {t('wallet_view.received')}
                          </Typography>
                          <SatsWithIcon amountMSats={settledIn} />
                        </Stack>
                      </Stack>
                    </Box>
                  </Stack>

                  <Divider />

                  <Grid container spacing={2}>
                    {[
                      {
                        title: t('wallet_view.pay_anything'),
                        icon: 'solar:plain-2-bold-duotone',
                        action: sendDrawer.onTrue,
                      },
                      {
                        title: t('wallet_view.request_anywhere'),
                        icon: 'solar:qr-code-bold-duotone',
                        action: receiveDrawer.onTrue,
                      },
                      {
                        title: t('wallet_view.identity'),
                        icon: 'solar:fingerprint-bold-duotone',
                        href: paths.identity,
                      },
                    ].map((item) => (
                      <Grid key={item.title} size={{ xs: 12, sm: 4 }}>
                        <Button
                          fullWidth
                          href={item.href}
                          color="inherit"
                          variant="outlined"
                          onClick={item.action}
                          startIcon={<Iconify icon={item.icon} />}
                          sx={{ justifyContent: 'flex-start', minHeight: 52 }}
                        >
                          {item.title}
                        </Button>
                      </Grid>
                    ))}
                  </Grid>
                </Stack>
              </Card>
            </Grid>

            <Grid size={{ xs: 12, lg: 4 }}>
              <Stack spacing={3}>
                <Card sx={{ p: 3, borderRadius: 1 }}>
                  <Stack spacing={2}>
                    <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                      <Typography variant="h6">{t('wallet_view.needs_action')}</Typography>
                      <Label color={failedPayments.length ? 'error' : 'success'}>
                        {failedPayments.length ? failedPayments.length : t('wallet_view.clear')}
                      </Label>
                    </Stack>

                    {failedPayments.length > 0 ? (
                      failedPayments.slice(0, 3).map((payment) => (
                        <Alert key={payment.id} severity="error" variant="outlined">
                          {payment.error || t('wallet_view.failed_payment')}
                        </Alert>
                      ))
                    ) : (
                      <Alert severity="success" variant="outlined">
                        {t('wallet_view.no_actions')}
                      </Alert>
                    )}

                    {pendingIncoming > 0 && (
                      <Alert severity="info" variant="outlined">
                        {t('wallet_view.pending_incoming')}
                      </Alert>
                    )}
                  </Stack>
                </Card>

                <Card sx={{ p: 3, borderRadius: 1 }}>
                  <Stack spacing={2}>
                    <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                      <Typography variant="h6">{t('wallet_view.people')}</Typography>
                      <Button href={paths.wallet.contacts} size="small" color="inherit">
                        {t('view_all')}
                      </Button>
                    </Stack>
                    <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                      {contacts.slice(0, 8).map((contact) => (
                        <Chip
                          key={contact.ln_address}
                          label={contact.ln_address.split('@')[0]}
                          variant="outlined"
                          onClick={() => {
                            sendDrawer.onTrue();
                          }}
                        />
                      ))}
                      {contacts.length === 0 && (
                        <Typography variant="body2" color="text.secondary">
                          {t('wallet_view.no_contacts')}
                        </Typography>
                      )}
                    </Stack>
                  </Stack>
                </Card>
              </Stack>
            </Grid>

            <Grid size={{ xs: 12 }}>
              <Card sx={{ p: 3, borderRadius: 1 }}>
                <Stack spacing={2}>
                  <Stack
                    direction={{ xs: 'column', sm: 'row' }}
                    spacing={1}
                    sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}
                  >
                    <Stack>
                      <Typography variant="h6">{t('wallet_view.recent_activity')}</Typography>
                      <Typography variant="body2" color="text.secondary">
                        {t('wallet_view.unified_activity')}
                      </Typography>
                    </Stack>
                    <Button
                      href={paths.activity}
                      color="inherit"
                      endIcon={<Iconify icon="eva:arrow-ios-forward-fill" />}
                    >
                      {t('view_all')}
                    </Button>
                  </Stack>

                  <Divider />

                  <Stack spacing={1}>
                    {allTransactions.slice(0, 8).map((tx) => (
                      <Box
                        key={`${tx.transaction_type}-${tx.id}`}
                        sx={[
                          (theme) => ({
                            p: 1.5,
                            gap: 1.5,
                            display: 'grid',
                            borderRadius: 1,
                            alignItems: 'center',
                            gridTemplateColumns: {
                              xs: '32px minmax(0, 1fr)',
                              sm: '32px minmax(0, 1fr) auto auto',
                            },
                            bgcolor: 'background.neutral',
                            border: `1px solid ${theme.vars.palette.divider}`,
                          }),
                        ]}
                      >
                        <Box
                          sx={{
                            width: 32,
                            height: 32,
                            display: 'grid',
                            borderRadius: 1,
                            placeItems: 'center',
                            color: txDirection(tx) === 'in' ? 'success.main' : 'warning.main',
                            bgcolor: txDirection(tx) === 'in' ? 'success.lighter' : 'warning.lighter',
                          }}
                        >
                          <Iconify icon={railIcon(tx)} />
                        </Box>
                        <Stack sx={{ minWidth: 0 }}>
                          <Typography variant="subtitle2" noWrap>
                            {tx.description || tx.id}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {fFromNow(tx.created_at)} · {tx.ledger}
                          </Typography>
                        </Stack>
                        <SatsWithIcon
                          amountMSats={txAmount(tx)}
                          sx={{ display: { xs: 'none', sm: 'block' } }}
                        />
                        <Label color={statusColor(tx.status)}>{tx.status}</Label>
                      </Box>
                    ))}

                    {allTransactions.length === 0 && (
                      <Box sx={{ py: 5, textAlign: 'center' }}>
                        <Iconify
                          width={48}
                          icon="solar:bill-list-bold-duotone"
                          sx={{ color: 'text.disabled' }}
                        />
                        <Typography variant="subtitle2" sx={{ mt: 1 }}>
                          {t('wallet_view.empty_activity')}
                        </Typography>
                      </Box>
                    )}
                  </Stack>
                </Stack>
              </Card>
            </Grid>
          </Grid>

          <SendMoneyDrawer
            open={sendDrawer.value}
            contacts={contacts}
            fiatPrices={prices}
            balance={wallet?.balance.available_msat}
            onClose={sendDrawer.onFalse}
            onSuccess={onMutation}
          />
          <ReceiveMoneyDrawer
            open={receiveDrawer.value}
            fiatPrices={prices}
            lnAddress={wallet?.ln_address}
            onClose={receiveDrawer.onFalse}
            onSuccess={onMutation}
          />
        </>
      )}
    </DashboardContent>
  );
}
