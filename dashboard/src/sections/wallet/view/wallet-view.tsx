'use client';

import type { Contact } from 'src/lib/swissknife';
import type { IFiatPrices } from 'src/types/bitcoin';
import type { ITransaction } from 'src/types/transaction';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';
import { useMemo, useState, useEffect } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Avatar from '@mui/material/Avatar';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import ButtonBase from '@mui/material/ButtonBase';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';

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
import { useActiveWallet } from 'src/actions/account-wallet';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { useSettingsContext } from 'src/components/settings';

import { TransactionQuickDrawer } from 'src/sections/transaction/transaction-quick-drawer';

import { TransactionType } from 'src/types/transaction';

import { SendMoneyDrawer, ReceiveMoneyDrawer } from '../money-drawers';

// ----------------------------------------------------------------------

const fallbackFiatPrices: IFiatPrices = { USD: 0, EUR: 0, CHF: 0 };
const recentActivityLimit = 8;

function txAmount(tx: ITransaction) {
  return (tx.amount_msat || 0) + (tx.fee_msat || 0);
}

function isOpenAmountRequest(tx: ITransaction) {
  return (
    tx.transaction_type === TransactionType.INVOICE && tx.status === 'Pending' && !tx.amount_msat
  );
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
    ? paths.wallet.payment(tx.id)
    : paths.wallet.invoice(tx.id);
}

function txNetworkLabel(tx: ITransaction) {
  if (tx.ledger === 'Onchain') return 'On-chain';
  if (tx.ledger === 'Internal') return 'Internal';
  return 'Lightning';
}

function txDisplayTitle(tx: ITransaction, t: (key: string) => string) {
  if (tx.description) return tx.description;

  if (tx.transaction_type === TransactionType.INVOICE) {
    if (isOpenAmountRequest(tx)) return t('wallet_view.open_amount_request');
    if (tx.status === 'Pending') return t('wallet_view.payment_request');
    if (tx.status === 'Expired') return t('wallet_view.request_expired');
    return t('wallet_view.payment_received');
  }

  if (tx.status === 'Pending') return t('wallet_view.payment_pending');
  if (tx.status === 'Failed') return t('wallet_view.payment_failed');
  return t('wallet_view.payment_sent');
}

// ----------------------------------------------------------------------

export function WalletView() {
  const router = useRouter();
  const sendDrawer = useBoolean();
  const receiveDrawer = useBoolean();
  const { t } = useTranslate();
  const settings = useSettingsContext();
  const { state } = settings;
  const [sendInitialInput, setSendInitialInput] = useState('');
  const [detailTransaction, setDetailTransaction] = useState<ITransaction | null>(null);
  const [lastWalletSyncAt, setLastWalletSyncAt] = useState<Date | null>(null);
  const [isRefreshingWallet, setIsRefreshingWallet] = useState(false);

  const { wallet, walletLoading, walletError } = useActiveWallet();
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

  const openAmountPendingInvoices = useMemo(
    () => pendingInvoices.filter((invoice) => !invoice.amount_msat),
    [pendingInvoices]
  );

  const pendingIncoming = useMemo(
    () =>
      pendingInvoices.reduce(
        (total, invoice) =>
          total + (invoice.amount_msat && invoice.amount_msat > 0 ? invoice.amount_msat : 0),
        0
      ),
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

  const expiredInvoices = useMemo(
    () => wallet?.invoices.filter((invoice) => invoice.status === 'Expired') ?? [],
    [wallet?.invoices]
  );

  const onMutation = () => {
    if (wallet?.id) mutate(endpointKeys.accountWallet.get(wallet.id));
  };
  const fiatValueAvailable = (prices[state.currency] ?? 0) > 0;
  const balancesHidden = state.hideBalances ?? false;
  const lnAddressDisplay = wallet?.ln_address?.username
    ? displayLnAddress(wallet.ln_address.username)
    : '';
  const needsActionCount = failedPayments.length + expiredInvoices.length;
  const inProgressCount = pendingInvoices.length + pendingPayments.length;
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
  const toggleBalances = () => {
    settings.setState({ hideBalances: !balancesHidden });
  };
  const refreshWallet = async () => {
    try {
      setIsRefreshingWallet(true);
      if (wallet?.id) await mutate(endpointKeys.accountWallet.get(wallet.id));
      setLastWalletSyncAt(new Date());
    } catch {
      toast.error(t('wallet_view.refresh_failed'));
    } finally {
      setIsRefreshingWallet(false);
    }
  };

  useEffect(() => {
    if (wallet && !walletLoading) {
      setLastWalletSyncAt(new Date());
    }
  }, [wallet, walletLoading]);

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
              {!wallet?.ln_address && (
                <Typography variant="body2" color="text.secondary">
                  {t('wallet_view.identity_missing')}
                </Typography>
              )}
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
                      <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                        <Typography variant="overline" color="text.secondary">
                          {t('wallet_view.spendable_now')}
                        </Typography>
                        <Tooltip
                          title={
                            balancesHidden
                              ? t('wallet_view.show_balances')
                              : t('wallet_view.hide_balances')
                          }
                        >
                          <IconButton
                            size="small"
                            onClick={toggleBalances}
                            aria-label={
                              balancesHidden
                                ? t('wallet_view.show_balances')
                                : t('wallet_view.hide_balances')
                            }
                          >
                            <Iconify
                              width={18}
                              icon={balancesHidden ? 'solar:eye-closed-bold' : 'solar:eye-bold'}
                            />
                          </IconButton>
                        </Tooltip>
                      </Stack>
                      <SatsWithIcon
                        variant="h2"
                        amountMSats={wallet?.balance.available_msat ?? 0}
                        sx={{ letterSpacing: 0, fontWeight: 400 }}
                      />
                      {fiatValueAvailable ? (
                        <Typography variant="subtitle1" color="text.secondary">
                          {balancesHidden
                            ? t('wallet_view.hidden_amount')
                            : fCurrency(
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
                      {(wallet?.balance.reserved_msat ?? 0) > 0 && (
                        <Typography variant="caption" color="text.secondary">
                          {t('wallet_view.reserved_amount')}{' '}
                          <SatsWithIcon
                            component="span"
                            amountMSats={wallet?.balance.reserved_msat ?? 0}
                          />
                        </Typography>
                      )}
                      {lastWalletSyncAt && (
                        <Stack
                          direction="row"
                          spacing={0.5}
                          sx={{ alignItems: 'center', color: 'text.disabled' }}
                        >
                          <Typography variant="caption" color="inherit">
                            {t('wallet_view.synced_ago', { time: fFromNow(lastWalletSyncAt) })}
                          </Typography>
                          <Tooltip title={t('wallet_view.refresh_wallet')}>
                            <span>
                              <IconButton
                                size="small"
                                onClick={refreshWallet}
                                disabled={isRefreshingWallet}
                                aria-label={t('wallet_view.refresh_wallet')}
                                sx={{
                                  p: 0.25,
                                  width: 24,
                                  height: 24,
                                  color: 'inherit',
                                  '&.Mui-disabled': { color: 'text.disabled' },
                                }}
                              >
                                <Iconify
                                  width={14}
                                  icon="solar:refresh-bold"
                                  sx={{
                                    '@keyframes walletRefreshSpin': {
                                      to: { transform: 'rotate(360deg)' },
                                    },
                                    ...(isRefreshingWallet && {
                                      animation: 'walletRefreshSpin 0.8s linear infinite',
                                    }),
                                  }}
                                />
                              </IconButton>
                            </span>
                          </Tooltip>
                        </Stack>
                      )}
                    </Stack>
                  </Stack>

                  <Divider />

                  <Grid container spacing={2}>
                    <Grid size={{ xs: 12, sm: 6 }}>
                      <WalletActionButton
                        onClick={openBlankSend}
                        title={t('wallet_view.send_money')}
                        icon="solar:arrow-up-linear"
                        color="warning"
                      />
                    </Grid>

                    <Grid size={{ xs: 12, sm: 6 }}>
                      <WalletActionButton
                        onClick={receiveDrawer.onTrue}
                        title={t('wallet_view.receive_money')}
                        detail={
                          pendingInvoices.length
                            ? t('wallet_view.active_requests_summary', {
                                count: pendingInvoices.length,
                              })
                            : t('wallet_view.no_active_requests')
                        }
                        icon="solar:arrow-down-linear"
                        color="success"
                      />
                    </Grid>
                  </Grid>

                  <Box
                    sx={[
                      (theme) => ({
                        p: 2,
                        borderRadius: 1,
                        bgcolor: 'background.neutral',
                        border: `1px solid ${theme.vars.palette.divider}`,
                      }),
                    ]}
                  >
                    <Stack
                      direction={{ xs: 'column', sm: 'row' }}
                      spacing={2}
                      sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}
                    >
                      <Stack
                        direction="row"
                        spacing={1.5}
                        sx={{ alignItems: 'center', minWidth: 0 }}
                      >
                        <Box
                          sx={{
                            width: 34,
                            height: 34,
                            display: 'grid',
                            borderRadius: 1,
                            placeItems: 'center',
                            color: wallet?.ln_address ? 'grey.900' : 'text.disabled',
                            bgcolor: wallet?.ln_address
                              ? 'warning.main'
                              : 'action.disabledBackground',
                          }}
                        >
                          <Iconify icon="solar:bolt-bold-duotone" width={21} />
                        </Box>
                        <Stack sx={{ minWidth: 0 }}>
                          <Typography variant="caption" color="text.secondary">
                            {t('wallet_view.reusable_receive')}
                          </Typography>
                          <Typography variant="subtitle2" noWrap>
                            {lnAddressDisplay || t('wallet_view.no_lightning_address')}
                          </Typography>
                        </Stack>
                      </Stack>
                      <Button
                        href={`${paths.identity}?tab=lightning`}
                        color="inherit"
                        variant="outlined"
                      >
                        {wallet?.ln_address
                          ? t('wallet_view.manage_receive_identity')
                          : t('wallet_view.claim_receive_identity')}
                      </Button>
                    </Stack>
                  </Box>
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
                      <Label color={needsActionCount ? 'warning' : 'success'}>
                        {needsActionCount || t('wallet_view.clear')}
                      </Label>
                    </Stack>

                    {failedPayments.length > 0 && (
                      <ButtonBase
                        onClick={() => router.push(paths.activityList('payment'))}
                        sx={[
                          (theme) => ({
                            p: 1.5,
                            gap: 1.5,
                            width: 1,
                            display: 'flex',
                            borderRadius: 1,
                            textAlign: 'left',
                            alignItems: 'center',
                            border: `1px solid ${theme.vars.palette.error.main}`,
                          }),
                        ]}
                      >
                        <Box
                          sx={{
                            width: 34,
                            height: 34,
                            display: 'grid',
                            flexShrink: 0,
                            borderRadius: 1,
                            placeItems: 'center',
                            color: 'grey.900',
                            bgcolor: 'error.main',
                          }}
                        >
                          <Iconify icon="solar:danger-triangle-bold-duotone" width={21} />
                        </Box>
                        <Stack sx={{ minWidth: 0, flex: 1 }}>
                          <Typography variant="subtitle2">
                            {t('wallet_view.failed_payments_action', {
                              count: failedPayments.length,
                            })}
                          </Typography>
                          <Typography variant="caption" color="text.secondary" noWrap>
                            {failedPayments[0]?.error || t('wallet_view.failed_payment')}
                          </Typography>
                        </Stack>
                      </ButtonBase>
                    )}

                    {expiredInvoices.length > 0 && (
                      <ButtonBase
                        onClick={() => router.push(paths.activityList('invoice'))}
                        sx={[
                          (theme) => ({
                            p: 1.5,
                            gap: 1.5,
                            width: 1,
                            display: 'flex',
                            borderRadius: 1,
                            textAlign: 'left',
                            alignItems: 'center',
                            border: `1px solid ${theme.vars.palette.warning.main}`,
                          }),
                        ]}
                      >
                        <Box
                          sx={{
                            width: 34,
                            height: 34,
                            display: 'grid',
                            flexShrink: 0,
                            borderRadius: 1,
                            placeItems: 'center',
                            color: 'grey.900',
                            bgcolor: 'warning.main',
                          }}
                        >
                          <Iconify icon="solar:bill-cross-bold-duotone" width={21} />
                        </Box>
                        <Stack sx={{ minWidth: 0, flex: 1 }}>
                          <Typography variant="subtitle2">
                            {t('wallet_view.expired_requests_action', {
                              count: expiredInvoices.length,
                            })}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {t('wallet_view.expired_requests_detail')}
                          </Typography>
                        </Stack>
                      </ButtonBase>
                    )}

                    {needsActionCount === 0 && (
                      <Stack spacing={0.5}>
                        <Typography variant="subtitle2">{t('wallet_view.no_actions')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('wallet_view.no_actions_detail')}
                        </Typography>
                      </Stack>
                    )}
                  </Stack>
                </Card>

                <Card sx={{ p: 3, borderRadius: 1 }}>
                  <Stack spacing={2}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                    >
                      <Typography variant="h6">{t('wallet_view.in_progress')}</Typography>
                      <Label color={inProgressCount ? 'info' : 'default'}>
                        {inProgressCount || t('wallet_view.none')}
                      </Label>
                    </Stack>

                    {pendingInvoices.length > 0 && (
                      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
                        <Box
                          sx={{
                            width: 34,
                            height: 34,
                            display: 'grid',
                            flexShrink: 0,
                            borderRadius: 1,
                            placeItems: 'center',
                            color: 'grey.900',
                            bgcolor: 'info.main',
                          }}
                        >
                          <Iconify icon="solar:download-minimalistic-bold-duotone" width={21} />
                        </Box>
                        <Stack sx={{ flex: 1, minWidth: 0 }}>
                          <Typography variant="subtitle2">
                            {t('wallet_view.active_requests_summary', {
                              count: pendingInvoices.length,
                            })}
                          </Typography>
                          <Stack spacing={0.25}>
                            {pendingIncoming > 0 && (
                              <Typography variant="caption" color="text.secondary">
                                <SatsWithIcon component="span" amountMSats={pendingIncoming} />{' '}
                                {t('wallet_view.fixed_requested')}
                              </Typography>
                            )}
                            {openAmountPendingInvoices.length > 0 && (
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.open_amount_requests_waiting', {
                                  count: openAmountPendingInvoices.length,
                                })}
                              </Typography>
                            )}
                            {pendingIncoming === 0 && openAmountPendingInvoices.length === 0 && (
                              <Typography variant="caption" color="text.secondary">
                                {t('wallet_view.requested_not_received')}
                              </Typography>
                            )}
                          </Stack>
                        </Stack>
                      </Stack>
                    )}

                    {pendingPayments.length > 0 && (
                      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
                        <Box
                          sx={{
                            width: 34,
                            height: 34,
                            display: 'grid',
                            flexShrink: 0,
                            borderRadius: 1,
                            placeItems: 'center',
                            color: 'grey.900',
                            bgcolor: 'warning.main',
                          }}
                        >
                          <Iconify icon="solar:upload-minimalistic-bold-duotone" width={21} />
                        </Box>
                        <Stack sx={{ flex: 1, minWidth: 0 }}>
                          <Typography variant="subtitle2">
                            {t('wallet_view.pending_outgoing_action', {
                              count: pendingPayments.length,
                            })}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {t('wallet_view.pending_outgoing_detail')}
                          </Typography>
                        </Stack>
                      </Stack>
                    )}

                    {inProgressCount === 0 && (
                      <Typography variant="body2" color="text.secondary">
                        {t('wallet_view.no_pending_activity')}
                      </Typography>
                    )}
                  </Stack>
                </Card>

                <Card sx={{ p: 3, borderRadius: 1 }}>
                  <Stack spacing={2}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                    >
                      <Stack spacing={0.25}>
                        <Typography variant="h6">{t('wallet_view.people')}</Typography>
                        <Typography variant="caption" color="text.secondary">
                          {t('wallet_view.saved_contacts', { count: contacts.length })}
                        </Typography>
                      </Stack>
                      <Button
                        href={paths.wallet.contacts}
                        size="small"
                        color="inherit"
                        endIcon={<Iconify icon="eva:arrow-ios-forward-fill" width={16} />}
                      >
                        {t('view_all')}
                      </Button>
                    </Stack>

                    <Stack spacing={2}>
                      {contacts.slice(0, 4).map((contact) => (
                        <ButtonBase
                          key={contact.ln_address}
                          onClick={() => openSendTo(contact.ln_address)}
                          aria-label={`${t('wallet_view.quick_send')}: ${contact.ln_address}`}
                          sx={[
                            (theme) => ({
                              p: 1,
                              ml: -1,
                              width: 1,
                              gap: 1.5,
                              display: 'flex',
                              borderRadius: 1,
                              textAlign: 'left',
                              alignItems: 'center',
                              justifyContent: 'flex-start',
                              '&:hover': { bgcolor: 'background.neutral' },
                              '&:focus-visible': {
                                outline: `2px solid ${theme.vars.palette.primary.main}`,
                                outlineOffset: 2,
                              },
                            }),
                          ]}
                        >
                          <Avatar alt={contact.ln_address} sx={{ width: 40, height: 40 }}>
                            {contact.ln_address.charAt(0).toUpperCase()}
                          </Avatar>

                          <ListItemText
                            primary={contact.ln_address}
                            secondary={fFromNow(contact.contact_since)}
                            slotProps={{
                              primary: { noWrap: true, sx: { typography: 'subtitle2' } },
                              secondary: { sx: { typography: 'caption', color: 'text.disabled' } },
                            }}
                          />

                          <Iconify
                            icon="eva:diagonal-arrow-right-up-fill"
                            sx={{ color: 'text.secondary', flexShrink: 0 }}
                          />
                        </ButtonBase>
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
                    direction="row"
                    spacing={1}
                    sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                  >
                    <Stack spacing={0.25}>
                      <Typography variant="h6">{t('wallet_view.recent_activity')}</Typography>
                      <Typography variant="body2" color="text.secondary">
                        {t('wallet_view.recent_activity_limit', { count: recentActivityLimit })}
                      </Typography>
                    </Stack>
                    <Button
                      href={paths.activity}
                      size="small"
                      color="inherit"
                      endIcon={<Iconify icon="eva:arrow-ios-forward-fill" width={16} />}
                    >
                      {t('view_all')}
                    </Button>
                  </Stack>

                  <Divider />

                  <Stack spacing={1}>
                    {allTransactions.slice(0, recentActivityLimit).map((tx) => {
                      const direction = txDirection(tx);
                      const isIncoming = direction === 'in';
                      const title = txDisplayTitle(tx, t);
                      const isOpenAmount = isOpenAmountRequest(tx);

                      return (
                        <Box
                          component={ButtonBase}
                          key={`${tx.transaction_type}-${tx.id}`}
                          aria-label={`${t('details')}: ${title}`}
                          onClick={() => setDetailTransaction(tx)}
                          sx={[
                            (theme) => ({
                              p: { xs: 1.25, sm: 1.5 },
                              gap: { xs: 1.25, sm: 1.5 },
                              display: 'grid',
                              width: 1,
                              borderRadius: 1,
                              textAlign: 'left',
                              alignItems: 'center',
                              gridTemplateColumns: {
                                xs: '32px minmax(0, 1fr) max-content',
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
                              bgcolor: isIncoming ? 'success.main' : 'warning.main',
                            }}
                          >
                            <Iconify icon={railIcon(tx)} sx={{ color: 'grey.900' }} />
                          </Box>

                          <Stack sx={{ minWidth: 0 }}>
                            <Stack
                              direction="row"
                              spacing={1}
                              sx={{ alignItems: 'center', minWidth: 0 }}
                            >
                              <Label color={isIncoming ? 'success' : 'warning'}>
                                {isIncoming
                                  ? t('wallet_view.direction_in')
                                  : t('wallet_view.direction_out')}
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
                            spacing={0.75}
                            sx={{
                              display: { xs: 'flex', sm: 'none' },
                              gridColumn: '3',
                              gridRow: '1',
                              alignItems: 'flex-end',
                              justifySelf: 'end',
                              minWidth: 86,
                            }}
                          >
                            {isOpenAmount ? (
                              <Typography variant="body2" color="text.secondary" noWrap>
                                {t('wallet_view.open_amount')}
                              </Typography>
                            ) : (
                              <Stack direction="row" spacing={0.25} sx={{ alignItems: 'center' }}>
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
                                  noWrap
                                  sx={{ color: isIncoming ? 'success.main' : 'warning.main' }}
                                />
                              </Stack>
                            )}

                            <Label color={statusColor(tx.status)}>{tx.status}</Label>
                          </Stack>

                          {isOpenAmount ? (
                            <Typography
                              variant="body2"
                              color="text.secondary"
                              sx={{
                                display: { xs: 'none', sm: 'block' },
                                gridColumn: { xs: '2', sm: 'auto' },
                                justifySelf: { xs: 'start', sm: 'end' },
                              }}
                            >
                              {t('wallet_view.open_amount')}
                            </Typography>
                          ) : (
                            <Stack
                              direction="row"
                              spacing={0.25}
                              sx={{
                                display: { xs: 'none', sm: 'flex' },
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
                          )}

                          <Label
                            color={statusColor(tx.status)}
                            sx={{
                              display: { xs: 'none', sm: 'inline-flex' },
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

          <Box sx={{ display: { xs: 'block', sm: 'none' }, height: 72 }} />
          <MobileWalletActions
            sendLabel={t('wallet_view.send_money')}
            receiveLabel={t('wallet_view.receive_money')}
            onSend={openBlankSend}
            onReceive={receiveDrawer.onTrue}
          />

          <TransactionQuickDrawer
            row={detailTransaction}
            detailHref={detailTransaction ? txHref(detailTransaction) : undefined}
            onClose={() => setDetailTransaction(null)}
          />
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

function MobileWalletActions({
  sendLabel,
  receiveLabel,
  onSend,
  onReceive,
}: {
  sendLabel: string;
  receiveLabel: string;
  onSend: VoidFunction;
  onReceive: VoidFunction;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          left: 0,
          right: 0,
          bottom: 0,
          gap: 1,
          px: 2,
          pt: 1.25,
          display: { xs: 'flex', sm: 'none' },
          position: 'fixed',
          zIndex: theme.zIndex.appBar,
          bgcolor: 'background.paper',
          borderTop: `1px solid ${theme.vars.palette.divider}`,
          pb: 'calc(10px + env(safe-area-inset-bottom))',
        }),
      ]}
    >
      <Button
        fullWidth
        color="inherit"
        variant="outlined"
        onClick={onSend}
        startIcon={<Iconify icon="solar:arrow-up-linear" />}
      >
        {sendLabel}
      </Button>
      <Button
        fullWidth
        color="inherit"
        variant="contained"
        onClick={onReceive}
        startIcon={<Iconify icon="solar:arrow-down-linear" />}
      >
        {receiveLabel}
      </Button>
    </Box>
  );
}

function WalletActionButton({
  title,
  detail,
  icon,
  color,
  onClick,
}: {
  title: string;
  detail?: string;
  icon: string;
  color: 'success' | 'warning';
  onClick: VoidFunction;
}) {
  return (
    <ButtonBase
      onClick={onClick}
      sx={[
        (theme) => ({
          p: 2,
          gap: 1.5,
          width: 1,
          minHeight: 108,
          display: 'flex',
          borderRadius: 1,
          textAlign: 'left',
          alignItems: 'center',
          justifyContent: 'space-between',
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
          transition: theme.transitions.create(['border-color', 'background-color']),
          '&:hover': {
            bgcolor: 'background.paper',
            borderColor: theme.vars.palette[color].main,
          },
        }),
      ]}
    >
      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center', minWidth: 0 }}>
        <Box
          sx={[
            (theme) => ({
              width: 46,
              height: 46,
              display: 'grid',
              flexShrink: 0,
              borderRadius: 1,
              placeItems: 'center',
              color: theme.vars.palette[color].main,
              bgcolor: 'background.paper',
              border: `1px solid ${theme.vars.palette[color].main}`,
            }),
          ]}
        >
          <Iconify icon={icon} width={26} />
        </Box>
        <Stack sx={{ minWidth: 0 }}>
          <Typography variant="h6">{title}</Typography>
          {detail && (
            <Typography variant="body2" color="text.secondary">
              {detail}
            </Typography>
          )}
        </Stack>
      </Stack>

      <Iconify icon="eva:arrow-ios-forward-fill" sx={{ color: 'text.disabled', flexShrink: 0 }} />
    </ButtonBase>
  );
}
