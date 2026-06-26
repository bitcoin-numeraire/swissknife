'use client';

import type { Contact } from 'src/lib/swissknife';
import type { IFiatPrices } from 'src/types/bitcoin';
import type { ITransaction } from 'src/types/transaction';

import { mutate } from 'swr';
import { sumBy } from 'es-toolkit';
import { useMemo, useState } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import ButtonBase from '@mui/material/ButtonBase';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { satsToFiat } from 'src/utils/fiat';
import { shouldFail } from 'src/utils/errors';
import { fFromNow } from 'src/utils/format-time';
import { displayLnAddress } from 'src/utils/lnurl';
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

const fallbackFiatPrices: IFiatPrices = { USD: 0, EUR: 0, CHF: 0 };

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

function txHref(tx: ITransaction) {
  return tx.transaction_type === TransactionType.PAYMENT
    ? paths.activityPayment(tx.id)
    : paths.activityInvoice(tx.id);
}

function txNetworkLabel(tx: ITransaction) {
  if (tx.ledger === 'Onchain') return 'On-chain';
  if (tx.ledger === 'Internal') return 'Internal';
  return 'Lightning';
}

// ----------------------------------------------------------------------

export function WalletView() {
  const router = useRouter();
  const sendDrawer = useBoolean();
  const receiveDrawer = useBoolean();
  const { t } = useTranslate();
  const { state } = useSettingsContext();
  const [sendInitialInput, setSendInitialInput] = useState('');

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

  const pendingInvoices = useMemo(
    () => wallet?.invoices.filter((invoice) => invoice.status === 'Pending') ?? [],
    [wallet?.invoices]
  );

  const pendingIncoming = useMemo(
    () => pendingInvoices.reduce((total, invoice) => total + (invoice.amount_msat || 0), 0),
    [pendingInvoices]
  );

  const pendingPayments = useMemo(
    () => wallet?.payments.filter((payment) => payment.status === 'Pending') ?? [],
    [wallet?.payments]
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
  const fiatValueAvailable = (prices[state.currency] ?? 0) > 0;
  const lnAddressDisplay = wallet?.ln_address?.username
    ? displayLnAddress(wallet.ln_address.username)
    : '';
  const reviewQueueCount = failedPayments.length + pendingInvoices.length + pendingPayments.length;
  const openBlankSend = () => {
    setSendInitialInput('');
    sendDrawer.onTrue();
  };
  const openSendTo = (value: string) => {
    setSendInitialInput(value);
    sendDrawer.onTrue();
  };
  const closeSend = () => {
    setSendInitialInput('');
    sendDrawer.onFalse();
  };

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
              <Typography variant="h3">{t('wallet_view.title')}</Typography>
              <Typography variant="body2" color="text.secondary">
                {wallet?.ln_address
                  ? t('wallet_view.identity_ready')
                  : t('wallet_view.identity_missing')}
              </Typography>
            </Stack>

            <Stack direction="row" spacing={1}>
              <Button
                color="inherit"
                variant="outlined"
                startIcon={<Iconify icon="solar:arrow-up-linear" />}
                onClick={openBlankSend}
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

          {!fiatPricesError && !fiatValueAvailable && (
            <Alert severity="info" sx={{ mb: 3 }}>
              {t('wallet_view.regtest_value_note')}
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
                      {fiatValueAvailable ? (
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
                      ) : (
                        <Typography variant="subtitle1" color="text.secondary">
                          {t('wallet_view.no_fiat_value')}
                        </Typography>
                      )}
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

                  <Stack spacing={1.5}>
                    <Typography variant="overline" color="text.secondary">
                      {t('wallet_view.money_status')}
                    </Typography>
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 12, sm: 4 }}>
                        <ButtonBase
                          onClick={receiveDrawer.onTrue}
                          sx={[
                            (theme) => ({
                              p: 2,
                              gap: 1.5,
                              width: 1,
                              minHeight: 190,
                              display: 'flex',
                              borderRadius: 1,
                              textAlign: 'left',
                              alignItems: 'stretch',
                              flexDirection: 'column',
                              justifyContent: 'space-between',
                              border: `1px solid ${theme.vars.palette.divider}`,
                              '&:hover': { borderColor: theme.vars.palette.text.secondary },
                            }),
                          ]}
                        >
                          <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                            <Iconify icon="solar:qr-code-bold-duotone" width={22} />
                            <Typography variant="subtitle2" sx={{ flexGrow: 1 }}>
                              {t('wallet_view.receive_readiness')}
                            </Typography>
                            <Label color={wallet?.ln_address ? 'success' : 'warning'}>
                              {wallet?.ln_address ? t('wallet_view.ready') : t('wallet_view.setup')}
                            </Label>
                          </Stack>

                          <Stack spacing={0.5} sx={{ minWidth: 0 }}>
                            <Typography variant="h6" noWrap>
                              {lnAddressDisplay || t('wallet_view.no_lightning_address')}
                            </Typography>
                            <Typography variant="body2" color="text.secondary">
                              {wallet?.ln_address
                                ? t('wallet_view.receive_ready_detail')
                                : t('wallet_view.receive_setup_detail')}
                            </Typography>
                          </Stack>

                          <Stack spacing={1}>
                            <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.pending_requests')}
                              </Typography>
                              <Typography variant="caption">{pendingInvoices.length}</Typography>
                            </Stack>
                            <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.expected')}
                              </Typography>
                              <SatsWithIcon amountMSats={pendingIncoming} />
                            </Stack>
                          </Stack>
                        </ButtonBase>
                      </Grid>

                      <Grid size={{ xs: 12, sm: 4 }}>
                        <ButtonBase
                          onClick={openBlankSend}
                          sx={[
                            (theme) => ({
                              p: 2,
                              gap: 1.5,
                              width: 1,
                              minHeight: 190,
                              display: 'flex',
                              borderRadius: 1,
                              textAlign: 'left',
                              alignItems: 'stretch',
                              flexDirection: 'column',
                              justifyContent: 'space-between',
                              border: `1px solid ${theme.vars.palette.divider}`,
                              '&:hover': { borderColor: theme.vars.palette.text.secondary },
                            }),
                          ]}
                        >
                          <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                            <Iconify icon="solar:plain-2-bold-duotone" width={22} />
                            <Typography variant="subtitle2" sx={{ flexGrow: 1 }}>
                              {t('wallet_view.spend_readiness')}
                            </Typography>
                            <Label
                              color={
                                (wallet?.balance.available_msat ?? 0) > 0 ? 'success' : 'default'
                              }
                            >
                              {(wallet?.balance.available_msat ?? 0) > 0
                                ? t('wallet_view.ready')
                                : t('wallet_view.empty')}
                            </Label>
                          </Stack>

                          <Stack spacing={0.5}>
                            <SatsWithIcon
                              variant="h6"
                              amountMSats={wallet?.balance.available_msat ?? 0}
                            />
                            <Typography variant="body2" color="text.secondary">
                              {contacts.length
                                ? t('wallet_view.saved_contacts', { count: contacts.length })
                                : t('wallet_view.no_contacts')}
                            </Typography>
                          </Stack>

                          <Stack spacing={1}>
                            <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.reserved')}
                              </Typography>
                              <SatsWithIcon amountMSats={wallet?.balance.reserved_msat ?? 0} />
                            </Stack>
                            <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.fees_paid')}
                              </Typography>
                              <SatsWithIcon amountMSats={wallet?.balance.fees_paid_msat ?? 0} />
                            </Stack>
                          </Stack>
                        </ButtonBase>
                      </Grid>

                      <Grid size={{ xs: 12, sm: 4 }}>
                        <ButtonBase
                          onClick={() => router.push(paths.activity)}
                          sx={[
                            (theme) => ({
                              p: 2,
                              gap: 1.5,
                              width: 1,
                              minHeight: 190,
                              display: 'flex',
                              borderRadius: 1,
                              textAlign: 'left',
                              alignItems: 'stretch',
                              flexDirection: 'column',
                              justifyContent: 'space-between',
                              border: `1px solid ${theme.vars.palette.divider}`,
                              '&:hover': { borderColor: theme.vars.palette.text.secondary },
                            }),
                          ]}
                        >
                          <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                            <Iconify icon="solar:bill-list-bold-duotone" width={22} />
                            <Typography variant="subtitle2" sx={{ flexGrow: 1 }}>
                              {t('wallet_view.review_queue')}
                            </Typography>
                            <Label color={reviewQueueCount ? 'warning' : 'success'}>
                              {reviewQueueCount || t('wallet_view.clear')}
                            </Label>
                          </Stack>

                          <Stack spacing={0.5}>
                            <Typography variant="h6">
                              {reviewQueueCount
                                ? t('wallet_view.review_items', { count: reviewQueueCount })
                                : t('wallet_view.no_actions')}
                            </Typography>
                            <Typography variant="body2" color="text.secondary">
                              {t('wallet_view.review_queue_detail')}
                            </Typography>
                          </Stack>

                          <Stack spacing={1}>
                            <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.failed_payments')}
                              </Typography>
                              <Typography variant="caption">{failedPayments.length}</Typography>
                            </Stack>
                            <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.pending_outgoing')}
                              </Typography>
                              <Typography variant="caption">{pendingPayments.length}</Typography>
                            </Stack>
                          </Stack>
                        </ButtonBase>
                      </Grid>
                    </Grid>
                  </Stack>
                </Stack>
              </Card>
            </Grid>

            <Grid size={{ xs: 12, lg: 4 }}>
              <Stack spacing={3}>
                <Card sx={{ p: 3, borderRadius: 1 }}>
                  <Stack spacing={2}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                    >
                      <Typography variant="h6">{t('wallet_view.needs_action')}</Typography>
                      <Label color={reviewQueueCount ? 'warning' : 'success'}>
                        {reviewQueueCount || t('wallet_view.clear')}
                      </Label>
                    </Stack>

                    {failedPayments.slice(0, 3).map((payment) => (
                      <Alert key={payment.id} severity="error" variant="outlined">
                        {payment.error || t('wallet_view.failed_payment')}
                      </Alert>
                    ))}

                    {pendingPayments.length > 0 && (
                      <Alert severity="warning" variant="outlined">
                        {t('wallet_view.pending_outgoing_notice')}
                      </Alert>
                    )}

                    {pendingInvoices.length > 0 && (
                      <Alert severity="info" variant="outlined">
                        {t('wallet_view.pending_incoming')}
                      </Alert>
                    )}

                    {reviewQueueCount === 0 && (
                      <Alert severity="success" variant="outlined">
                        {t('wallet_view.no_actions')}
                      </Alert>
                    )}
                  </Stack>
                </Card>

                <Card sx={{ p: 3, borderRadius: 1 }}>
                  <Stack spacing={2}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                    >
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
                          onClick={() => openSendTo(contact.ln_address)}
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
                    {allTransactions.slice(0, 8).map((tx) => {
                      const direction = txDirection(tx);
                      const isIncoming = direction === 'in';
                      const title =
                        tx.description ||
                        (isIncoming ? t('activity_view.received') : t('activity_view.sent'));

                      return (
                        <Box
                          component={ButtonBase}
                          key={`${tx.transaction_type}-${tx.id}`}
                          aria-label={`${t('details')}: ${title}`}
                          onClick={() => router.push(txHref(tx))}
                          sx={[
                            (theme) => ({
                              p: 1.5,
                              gap: 1.5,
                              display: 'grid',
                              width: 1,
                              borderRadius: 1,
                              textAlign: 'left',
                              alignItems: 'center',
                              gridTemplateColumns: {
                                xs: '32px minmax(0, 1fr)',
                                sm: '32px minmax(0, 1fr) minmax(120px, auto) auto 20px',
                              },
                              bgcolor: 'background.neutral',
                              border: `1px solid ${theme.vars.palette.divider}`,
                              transition: theme.transitions.create([
                                'border-color',
                                'background-color',
                              ]),
                              '&:hover': {
                                bgcolor: 'background.paper',
                                borderColor: theme.vars.palette.text.secondary,
                              },
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
                              color: isIncoming ? 'success.main' : 'warning.main',
                              bgcolor: isIncoming ? 'success.lighter' : 'warning.lighter',
                            }}
                          >
                            <Iconify icon={railIcon(tx)} />
                          </Box>

                          <Stack sx={{ minWidth: 0 }}>
                            <Stack
                              direction="row"
                              spacing={1}
                              sx={{ alignItems: 'center', minWidth: 0 }}
                            >
                              <Label color={isIncoming ? 'success' : 'warning'}>
                                {isIncoming
                                  ? t('wallet_view.direction_received')
                                  : t('wallet_view.direction_sent')}
                              </Label>
                              <Typography variant="subtitle2" noWrap>
                                {title}
                              </Typography>
                            </Stack>
                            <Typography variant="caption" color="text.secondary">
                              {fFromNow(tx.created_at)} · {txNetworkLabel(tx)}
                            </Typography>
                          </Stack>

                          <Stack
                            direction="row"
                            spacing={0.25}
                            sx={{
                              gridColumn: { xs: '2', sm: 'auto' },
                              alignItems: 'center',
                              justifyContent: { xs: 'flex-start', sm: 'flex-end' },
                            }}
                          >
                            <Typography
                              component="span"
                              variant="body2"
                              sx={{ color: isIncoming ? 'success.main' : 'warning.main' }}
                            >
                              {isIncoming ? '+' : '-'}
                            </Typography>
                            <SatsWithIcon
                              component="span"
                              amountMSats={txAmount(tx)}
                              sx={{ color: isIncoming ? 'success.main' : 'warning.main' }}
                            />
                          </Stack>

                          <Label
                            color={statusColor(tx.status)}
                            sx={{
                              gridColumn: { xs: '2', sm: 'auto' },
                              justifySelf: { xs: 'start', sm: 'auto' },
                            }}
                          >
                            {tx.status}
                          </Label>

                          <Iconify
                            icon="eva:arrow-ios-forward-fill"
                            sx={{ display: { xs: 'none', sm: 'block' }, color: 'text.disabled' }}
                          />
                        </Box>
                      );
                    })}

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
            onClose={closeSend}
            onSuccess={onMutation}
            initialInput={sendInitialInput}
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
