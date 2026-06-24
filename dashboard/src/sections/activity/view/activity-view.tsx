'use client';

import type { LabelColor } from 'src/components/label';
import type { ITransaction } from 'src/types/transaction';

import { useMemo, useState } from 'react';

import Tab from '@mui/material/Tab';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Stack from '@mui/material/Stack';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';

import { shouldFail } from 'src/utils/errors';
import { fDateTime } from 'src/utils/format-time';
import { mergeAndSortTransactions } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetUserWallet } from 'src/actions/user-wallet';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

type ActivityLens = 'all' | 'in' | 'out' | 'pending' | 'failed';

function txDirection(tx: ITransaction) {
  return tx.transaction_type === TransactionType.INVOICE ? 'in' : 'out';
}

function txAmount(tx: ITransaction) {
  return (tx.amount_msat || 0) + (tx.fee_msat || 0);
}

function statusColor(status: string): LabelColor {
  if (status === 'Settled') return 'success';
  if (status === 'Failed' || status === 'Expired') return 'error';
  return 'warning';
}

function railIcon(tx: ITransaction) {
  if (tx.ledger === 'Onchain') return 'solar:link-bold-duotone';
  if (tx.ledger === 'Internal') return 'solar:home-angle-bold-duotone';
  return 'solar:bolt-bold-duotone';
}

function technicalPayload(tx: ITransaction) {
  return JSON.stringify(
    {
      id: tx.id,
      type: tx.transaction_type,
      ledger: tx.ledger,
      status: tx.status,
      wallet_id: tx.wallet_id,
      amount_msat: tx.amount_msat,
      fee_msat: tx.fee_msat,
      created_at: tx.created_at,
      payment_time: tx.payment_time,
    },
    null,
    2
  );
}

// ----------------------------------------------------------------------

export function ActivityView() {
  const { t } = useTranslate();
  const [lens, setLens] = useState<ActivityLens>('all');
  const [selected, setSelected] = useState<ITransaction>();

  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const errors = [walletError];
  const data = [wallet];
  const isLoading = [walletLoading];
  const failed = shouldFail(errors, data, isLoading);

  const transactions = useMemo(
    () => mergeAndSortTransactions(wallet?.invoices || [], wallet?.payments || []),
    [wallet?.invoices, wallet?.payments]
  );

  const filtered = useMemo(
    () =>
      transactions.filter((tx) => {
        if (lens === 'in') return txDirection(tx) === 'in';
        if (lens === 'out') return txDirection(tx) === 'out';
        if (lens === 'pending') return tx.status === 'Pending';
        if (lens === 'failed') return tx.status === 'Failed' || tx.status === 'Expired';
        return true;
      }),
    [lens, transactions]
  );

  const tabs: { value: ActivityLens; label: string; count: number }[] = [
    { value: 'all', label: t('activity_view.all'), count: transactions.length },
    { value: 'in', label: t('activity_view.in'), count: transactions.filter((tx) => txDirection(tx) === 'in').length },
    { value: 'out', label: t('activity_view.out'), count: transactions.filter((tx) => txDirection(tx) === 'out').length },
    { value: 'pending', label: t('activity_view.pending'), count: transactions.filter((tx) => tx.status === 'Pending').length },
    {
      value: 'failed',
      label: t('activity_view.needs_action'),
      count: transactions.filter((tx) => tx.status === 'Failed' || tx.status === 'Expired').length,
    },
  ];

  return (
    <DashboardContent maxWidth="xl">
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('activity')}
            links={[{ name: t('money') }, { name: t('activity') }]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <Card sx={{ borderRadius: 1 }}>
            <Tabs
              value={lens}
              onChange={(_, value) => setLens(value)}
              variant="scrollable"
              sx={{ px: 2, borderBottom: (theme) => `1px solid ${theme.vars.palette.divider}` }}
            >
              {tabs.map((tab) => (
                <Tab
                  key={tab.value}
                  value={tab.value}
                  label={tab.label}
                  iconPosition="end"
                  icon={<Label color={tab.count ? 'info' : 'default'}>{tab.count}</Label>}
                />
              ))}
            </Tabs>

            <Stack spacing={0} divider={<Divider />}>
              {filtered.map((tx) => (
                <Box
                  key={`${tx.transaction_type}-${tx.id}`}
                  component="button"
                  onClick={() => setSelected(tx)}
                  sx={[
                    (theme) => ({
                      p: 2,
                      gap: 2,
                      border: 0,
                      width: 1,
                      display: 'grid',
                      cursor: 'pointer',
                      textAlign: 'left',
                      alignItems: 'center',
                      bgcolor: 'transparent',
                      color: 'text.primary',
                      gridTemplateColumns: {
                        xs: '36px minmax(0, 1fr)',
                        md: '36px minmax(0, 1fr) 160px 140px 112px',
                      },
                      '&:hover': { bgcolor: theme.vars.palette.action.hover },
                    }),
                  ]}
                >
                  <Box
                    sx={{
                      width: 36,
                      height: 36,
                      display: 'grid',
                      borderRadius: 1,
                      placeItems: 'center',
                      color: txDirection(tx) === 'in' ? 'success.main' : 'warning.main',
                      bgcolor: txDirection(tx) === 'in' ? 'success.lighter' : 'warning.lighter',
                    }}
                  >
                    <Iconify icon={railIcon(tx)} width={20} />
                  </Box>

                  <Stack sx={{ minWidth: 0 }}>
                    <Typography variant="subtitle2" noWrap>
                      {tx.description || tx.id}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {fDateTime(tx.created_at)} · {tx.ledger}
                    </Typography>
                  </Stack>

                  <Typography variant="body2" color="text.secondary" sx={{ display: { xs: 'none', md: 'block' } }}>
                    {tx.wallet_id}
                  </Typography>
                  <SatsWithIcon amountMSats={txAmount(tx)} sx={{ display: { xs: 'none', md: 'block' } }} />
                  <Label color={statusColor(tx.status)}>{tx.status}</Label>
                </Box>
              ))}

              {filtered.length === 0 && (
                <EmptyContent
                  title={t('activity_view.empty_title')}
                  description={t('activity_view.empty_description')}
                  sx={{ py: 8 }}
                />
              )}
            </Stack>
          </Card>

          <Drawer
            anchor="right"
            open={!!selected}
            onClose={() => setSelected(undefined)}
            slotProps={{ paper: { sx: { width: { xs: 1, sm: 520 }, maxWidth: 1 } } }}
          >
            {selected && (
              <Stack spacing={3} sx={{ p: 3 }}>
                <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                  <Stack>
                    <Typography variant="h6">
                      {selected.transaction_type === TransactionType.INVOICE
                        ? t('activity_view.received')
                        : t('activity_view.sent')}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {selected.id}
                    </Typography>
                  </Stack>
                  <IconButton onClick={() => setSelected(undefined)}>
                    <Iconify icon="mingcute:close-line" />
                  </IconButton>
                </Stack>

                <SatsWithIcon amountMSats={txAmount(selected)} variant="h3" />
                <Divider />

                {[
                  [t('activity_view.rail'), selected.ledger],
                  [t('activity_view.status'), selected.status],
                  [t('activity_view.created'), fDateTime(selected.created_at)],
                  [t('activity_view.settled'), fDateTime(selected.payment_time)],
                  [t('activity_view.wallet'), selected.wallet_id],
                ].map(([label, value]) => (
                  <Stack key={label} direction="row" sx={{ justifyContent: 'space-between', gap: 2 }}>
                    <Typography variant="body2" color="text.secondary">
                      {label}
                    </Typography>
                    <Typography variant="body2" sx={{ textAlign: 'right' }}>
                      {value || '—'}
                    </Typography>
                  </Stack>
                ))}

                <Box
                  component="pre"
                  sx={{
                    m: 0,
                    p: 2,
                    overflow: 'auto',
                    borderRadius: 1,
                    typography: 'caption',
                    bgcolor: 'grey.900',
                    color: 'grey.100',
                  }}
                >
                  {technicalPayload(selected)}
                </Box>

                <Stack direction="row" sx={{ justifyContent: 'flex-end' }}>
                  <CopyButton value={technicalPayload(selected)} title={t('activity_view.copy_technical')} />
                </Stack>
              </Stack>
            )}
          </Drawer>
        </>
      )}
    </DashboardContent>
  );
}
