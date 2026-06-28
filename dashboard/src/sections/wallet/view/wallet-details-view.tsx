'use client';

import type { ReactNode } from 'react';
import type { BtcAddress, Invoice, Payment, Wallet } from 'src/lib/swissknife';

import { useMemo } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';

import { paths } from 'src/routes/paths';

import { fDate } from 'src/utils/format-time';
import { shouldFail } from 'src/utils/errors';
import { displayLnAddress } from 'src/utils/lnurl';
import { getLedgerLabel } from 'src/utils/transactions';
import { compactBitcoinAddress } from 'src/utils/bitcoin-request';
import { bitcoinAddressExplorerUrl } from 'src/utils/bitcoin-explorer';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useGetWallet } from 'src/actions/wallet';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListBtcAddresses } from 'src/actions/btc-addresses';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import {
  DetailRow,
  DetailCard,
  MetricTile,
  statusColor,
  formatDateTime,
} from 'src/sections/transaction/transaction-detail-common';

// ----------------------------------------------------------------------

type Props = {
  id: string;
};

type WalletTransaction = {
  id: string;
  kind: 'invoice' | 'payment';
  description?: string | null;
  amountMSats: number;
  createdAt: Date;
  ledger: string;
  status: string;
};

function walletName(wallet: Wallet) {
  if (wallet.ln_address?.username) return displayLnAddress(wallet.ln_address.username);
  return wallet.user_id;
}

function addressTypeLabel(address: BtcAddress) {
  if (address.address_type === 'p2tr') return 'Taproot';
  if (address.address_type === 'p2wpkh') return 'Native SegWit';
  if (address.address_type === 'p2pkh') return 'Legacy';
  return address.address_type.toUpperCase();
}

function StatTile({
  label,
  amountMSats,
  helper,
}: {
  label: string;
  amountMSats: number;
  helper?: string;
}) {
  return <MetricTile title={label} amountMSats={amountMSats} helper={helper} />;
}

function SummaryCount({ label, value }: { label: string; value: ReactNode }) {
  return (
    <Stack spacing={0.5}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="h6">{value}</Typography>
    </Stack>
  );
}

function getTransactions(wallet: Wallet): WalletTransaction[] {
  const invoices: WalletTransaction[] = wallet.invoices.map((invoice: Invoice) => ({
    id: invoice.id,
    kind: 'invoice',
    description: invoice.description,
    amountMSats: invoice.amount_received_msat ?? invoice.amount_msat ?? 0,
    createdAt: invoice.created_at,
    ledger: invoice.ledger,
    status: invoice.status,
  }));

  const payments: WalletTransaction[] = wallet.payments.map((payment: Payment) => ({
    id: payment.id,
    kind: 'payment',
    description: payment.description,
    amountMSats: payment.amount_msat + (payment.fee_msat ?? 0),
    createdAt: payment.created_at,
    ledger: payment.ledger,
    status: payment.status,
  }));

  return [...invoices, ...payments].sort(
    (left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime()
  );
}

function BitcoinAddressRow({ address }: { address: BtcAddress }) {
  const { t } = useTranslate();
  const explorerUrl = bitcoinAddressExplorerUrl(address.address);

  return (
    <Stack
      direction={{ xs: 'column', sm: 'row' }}
      spacing={1.5}
      sx={{ alignItems: { sm: 'center' } }}
    >
      <Stack spacing={0.5} sx={{ minWidth: 0, flex: 1 }}>
        <Stack
          direction="row"
          spacing={1}
          useFlexGap
          sx={{ alignItems: 'center', flexWrap: 'wrap' }}
        >
          <Typography
            variant="body2"
            sx={{ fontFamily: 'monospace', minWidth: 0, wordBreak: 'break-word' }}
          >
            {compactBitcoinAddress(address.address)}
          </Typography>
          <CopyButton value={address.address} title={t('copy')} />
          {explorerUrl && (
            <Tooltip title={t('transaction_actions.open_explorer')}>
              <IconButton
                size="small"
                component="a"
                href={explorerUrl}
                target="_blank"
                rel="noopener noreferrer"
              >
                <Iconify icon="solar:map-arrow-right-bold" width={18} />
              </IconButton>
            </Tooltip>
          )}
        </Stack>
        <Typography variant="caption" color="text.secondary">
          {addressTypeLabel(address)} · {fDate(address.created_at)}
        </Typography>
      </Stack>

      <Label variant="soft" color={address.used ? 'warning' : 'success'}>
        {address.used ? t('wallet_details.used') : t('wallet_details.unused')}
      </Label>
    </Stack>
  );
}

function RecentTransactionRow({ transaction }: { transaction: WalletTransaction }) {
  const { t } = useTranslate();
  const isPayment = transaction.kind === 'payment';
  const detailHref = isPayment
    ? paths.admin.payment(transaction.id)
    : paths.admin.invoice(transaction.id);
  const amountColor = isPayment ? 'warning.main' : 'success.main';

  return (
    <Button
      fullWidth
      href={detailHref}
      color="inherit"
      sx={{
        p: 0,
        justifyContent: 'stretch',
        textAlign: 'left',
        borderRadius: 1,
      }}
    >
      <Stack
        direction={{ xs: 'column', sm: 'row' }}
        spacing={1.5}
        sx={{
          p: 1.5,
          width: 1,
          alignItems: { sm: 'center' },
          borderRadius: 1,
          bgcolor: 'background.neutral',
        }}
      >
        <Stack spacing={0.5} sx={{ minWidth: 0, flex: 1 }}>
          <Stack
            direction="row"
            spacing={1}
            useFlexGap
            sx={{ alignItems: 'center', flexWrap: 'wrap' }}
          >
            <Label variant="soft" color={isPayment ? 'warning' : 'success'}>
              {isPayment ? 'Out' : 'In'}
            </Label>
            <Typography variant="subtitle2" noWrap>
              {transaction.description || t('recent_transactions.empty_description')}
            </Typography>
          </Stack>
          <Typography variant="caption" color="text.secondary">
            {formatDateTime(transaction.createdAt)} · {getLedgerLabel(transaction.ledger, t)}
          </Typography>
        </Stack>

        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <Typography component="span" variant="body2" sx={{ color: amountColor }}>
            {isPayment ? '-' : '+'}
          </Typography>
          <SatsWithIcon
            amountMSats={transaction.amountMSats}
            variant="body2"
            sx={{ color: amountColor }}
          />
          <Label variant="soft" color={statusColor(transaction.status)}>
            {transaction.status}
          </Label>
        </Stack>
      </Stack>
    </Button>
  );
}

export function WalletDetailsView({ id }: Props) {
  const { t } = useTranslate();
  const { wallet, walletLoading, walletError } = useGetWallet(id);
  const { btcAddresses, btcAddressesLoading, btcAddressesError } = useListBtcAddresses({
    wallet_id: id,
  });

  const errors = [walletError, btcAddressesError];
  const data = [wallet, btcAddresses];
  const isLoading = [walletLoading, btcAddressesLoading];
  const failed = shouldFail(errors, data, isLoading);

  const transactions = useMemo(() => (wallet ? getTransactions(wallet) : []), [wallet]);

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_WALLET]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={walletName(wallet!)}
              links={[
                { name: t('admin') },
                { name: t('wallets'), href: paths.admin.wallets },
                { name: t('details') },
              ]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Stack spacing={3}>
              <Card sx={{ p: 3, borderRadius: 1 }}>
                <Grid container spacing={3} sx={{ alignItems: 'stretch' }}>
                  <Grid size={{ xs: 12, md: 5 }}>
                    <Stack spacing={2} sx={{ height: 1, justifyContent: 'center' }}>
                      <Box
                        sx={(theme) => ({
                          width: 56,
                          height: 56,
                          display: 'grid',
                          borderRadius: 1,
                          placeItems: 'center',
                          color: theme.vars.palette.warning.main,
                          bgcolor: 'background.paper',
                          border: `1px solid ${theme.vars.palette.warning.main}`,
                        })}
                      >
                        <Iconify icon="solar:safe-square-bold-duotone" width={32} />
                      </Box>

                      <Stack spacing={1}>
                        <Stack
                          direction="row"
                          spacing={1}
                          useFlexGap
                          sx={{ alignItems: 'center', flexWrap: 'wrap' }}
                        >
                          <Typography variant="h4">{walletName(wallet!)}</Typography>
                          <Label
                            variant="soft"
                            color={wallet!.ln_address?.active ? 'success' : 'default'}
                          >
                            {wallet!.ln_address?.active
                              ? t('accounts_view.ready')
                              : t('accounts_view.missing')}
                          </Label>
                        </Stack>
                        <Stack
                          direction="row"
                          spacing={0.5}
                          sx={{ alignItems: 'center', minWidth: 0 }}
                        >
                          <Typography
                            variant="body2"
                            color="text.secondary"
                            sx={{ fontFamily: 'monospace', wordBreak: 'break-word' }}
                          >
                            {wallet!.id}
                          </Typography>
                          <CopyButton value={wallet!.id} title={t('copy')} />
                        </Stack>
                      </Stack>
                    </Stack>
                  </Grid>

                  <Grid size={{ xs: 12, md: 7 }}>
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <StatTile
                          label={t('wallet_details.spendable')}
                          amountMSats={wallet!.balance.available_msat}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <StatTile
                          label={t('wallet_details.received')}
                          amountMSats={wallet!.balance.received_msat}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <StatTile
                          label={t('wallet_details.sent')}
                          amountMSats={wallet!.balance.sent_msat}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <StatTile
                          label={t('wallet_details.fees_paid')}
                          amountMSats={wallet!.balance.fees_paid_msat}
                        />
                      </Grid>
                    </Grid>
                  </Grid>
                </Grid>
              </Card>

              <Grid container spacing={3}>
                <Grid size={{ xs: 12, md: 5 }}>
                  <DetailCard
                    title={t('wallet_details.identity')}
                    icon="solar:user-id-bold-duotone"
                    color="info"
                  >
                    <DetailRow
                      label={t('wallet_details.user')}
                      value={wallet!.user_id}
                      copyValue={wallet!.user_id}
                    />
                    <DetailRow
                      label={t('wallet_list.created')}
                      value={formatDateTime(wallet!.created_at)}
                    />
                    <DetailRow
                      label={t('wallet_details.updated')}
                      value={formatDateTime(wallet!.updated_at)}
                    />
                    <DetailRow
                      label={t('lightning_address')}
                      value={
                        wallet!.ln_address?.username
                          ? displayLnAddress(wallet!.ln_address.username)
                          : undefined
                      }
                      copyValue={
                        wallet!.ln_address?.username
                          ? displayLnAddress(wallet!.ln_address.username)
                          : undefined
                      }
                      href={
                        wallet!.ln_address?.id
                          ? paths.admin.lnAddress(wallet!.ln_address.id)
                          : undefined
                      }
                      hrefLabel={t('details')}
                      targetBlank={false}
                    />
                    <Stack
                      direction={{ xs: 'column', sm: 'row' }}
                      spacing={1.5}
                      sx={{ alignItems: { sm: 'center' } }}
                    >
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ width: { sm: 180 }, flexShrink: 0 }}
                      >
                        {t('wallet_details.reserved')}
                      </Typography>
                      <SatsWithIcon amountMSats={wallet!.balance.reserved_msat} variant="body2" />
                    </Stack>
                  </DetailCard>
                </Grid>

                <Grid size={{ xs: 12, md: 7 }}>
                  <DetailCard
                    title={t('wallet_details.counts')}
                    icon="solar:chart-square-bold-duotone"
                    color="success"
                  >
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 4 }}>
                        <SummaryCount
                          label={t('wallet_list.invoices')}
                          value={wallet!.invoices.length}
                        />
                      </Grid>
                      <Grid size={{ xs: 4 }}>
                        <SummaryCount
                          label={t('wallet_list.payments')}
                          value={wallet!.payments.length}
                        />
                      </Grid>
                      <Grid size={{ xs: 4 }}>
                        <SummaryCount
                          label={t('wallet_list.contacts')}
                          value={wallet!.contacts.length}
                        />
                      </Grid>
                    </Grid>
                  </DetailCard>
                </Grid>

                <Grid size={{ xs: 12, md: 6 }}>
                  <DetailCard
                    title={t('wallet_details.recent_transactions')}
                    icon="solar:bill-list-bold-duotone"
                    color="warning"
                  >
                    {transactions.length ? (
                      <Stack spacing={1}>
                        {transactions.slice(0, 8).map((transaction) => (
                          <RecentTransactionRow
                            key={`${transaction.kind}-${transaction.id}`}
                            transaction={transaction}
                          />
                        ))}
                      </Stack>
                    ) : (
                      <EmptyContent title={t('wallet_details.no_transactions')} sx={{ py: 3 }} />
                    )}
                  </DetailCard>
                </Grid>

                <Grid size={{ xs: 12, md: 6 }}>
                  <DetailCard
                    title={t('wallet_details.bitcoin_addresses')}
                    icon="solar:link-round-angle-bold-duotone"
                    color="success"
                  >
                    {btcAddresses!.length ? (
                      <Stack
                        spacing={2}
                        divider={<Divider flexItem sx={{ borderStyle: 'dashed' }} />}
                      >
                        {btcAddresses!.map((address) => (
                          <BitcoinAddressRow key={address.id} address={address} />
                        ))}
                      </Stack>
                    ) : (
                      <EmptyContent
                        title={t('wallet_details.no_bitcoin_addresses')}
                        sx={{ py: 3 }}
                      />
                    )}
                  </DetailCard>
                </Grid>
              </Grid>
            </Stack>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
