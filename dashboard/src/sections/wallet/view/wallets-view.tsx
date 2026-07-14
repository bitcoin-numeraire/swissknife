'use client';

import type { TFunction } from 'i18next';
import type { LabelColor } from 'src/components/label';
import type { WalletOverview } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useMemo, useState } from 'react';
import { useBoolean, usePopover } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Link from '@mui/material/Link';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import MenuList from '@mui/material/MenuList';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import { alpha, useTheme } from '@mui/material/styles';
import TableContainer from '@mui/material/TableContainer';
import InputAdornment from '@mui/material/InputAdornment';

import { paths } from 'src/routes/paths';
import { useSearchParams } from 'src/routes/hooks';
import { RouterLink } from 'src/routes/components';

import { fDate } from 'src/utils/format-time';
import { displayLnAddress } from 'src/utils/lnurl';
import { shouldFail, handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletOverviews } from 'src/actions/wallet';
import { Permission, deleteWallet } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ItemAnalytic } from 'src/components/analytic';
import { ErrorView } from 'src/components/error/error-view';
import { RegisterWalletDrawer } from 'src/components/wallet';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import {
  useTable,
  emptyRows,
  TableNoData,
  TableEmptyRows,
  TableHeadCustom,
  TablePaginationCustom,
} from 'src/components/table';

import { RoleBasedGuard } from 'src/auth/guard';

import { WalletDetailsView } from './wallet-details-view';

// ----------------------------------------------------------------------

export type WalletReadiness = 'all' | 'ready' | 'paused' | 'missing';
export type WalletSort = 'activity' | 'recent' | 'network' | 'account';

const tableHead = (t: TFunction) => [
  { id: 'wallet', label: t('wallet') },
  { id: 'account', label: t('accounts_view.account') },
  { id: 'asset', label: t('accounts_view.asset') },
  { id: 'balance', label: t('wallet_list.balance'), align: 'right' as const },
  { id: 'identity', label: t('lightning_address') },
  { id: 'activity', label: t('wallets_view.activity') },
  { id: 'created', label: t('wallet_list.created') },
  { id: '', width: 56 },
];

export function walletReadiness(wallet: WalletOverview): Exclude<WalletReadiness, 'all'> {
  if (wallet.ln_address?.active) return 'ready';
  if (wallet.ln_address) return 'paused';
  return 'missing';
}

export function walletMatchesSearch(wallet: WalletOverview, query: string) {
  const normalizedQuery = query.trim().toLowerCase();

  if (!normalizedQuery) return true;

  const lnAddress = wallet.ln_address?.username ? displayLnAddress(wallet.ln_address.username) : '';

  return [
    wallet.id,
    wallet.account_id,
    wallet.label,
    lnAddress,
    wallet.asset?.code,
    wallet.asset?.display_ticker,
    wallet.asset?.network,
    wallet.asset?.protocol,
  ]
    .filter(Boolean)
    .some((value) => value!.toLowerCase().includes(normalizedQuery));
}

export function sortWallets(wallets: WalletOverview[], sort: WalletSort) {
  return [...wallets].sort((left, right) => {
    if (sort === 'recent') {
      return new Date(right.created_at).getTime() - new Date(left.created_at).getTime();
    }

    if (sort === 'network') {
      return (left.asset?.network ?? '').localeCompare(right.asset?.network ?? '');
    }

    if (sort === 'account') {
      return left.account_id.localeCompare(right.account_id);
    }

    return (
      right.n_invoices +
      right.n_payments +
      right.n_contacts -
      (left.n_invoices + left.n_payments + left.n_contacts)
    );
  });
}

function compactId(id: string) {
  return id.length > 20 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;
}

function walletName(wallet: WalletOverview) {
  if (wallet.label) return wallet.label;
  if (wallet.ln_address?.username) return displayLnAddress(wallet.ln_address.username);
  if (wallet.asset) return `${wallet.asset.display_ticker} · ${wallet.asset.network}`;
  return compactId(wallet.id);
}

function readinessColor(readiness: Exclude<WalletReadiness, 'all'>): LabelColor {
  if (readiness === 'ready') return 'success';
  if (readiness === 'paused') return 'warning';
  return 'default';
}

function WalletActions({ wallet }: { wallet: WalletOverview }) {
  const { t } = useTranslate();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  const handleDelete = async () => {
    isDeleting.onTrue();
    try {
      await deleteWallet({ path: { id: wallet.id } });
      await mutate(endpointKeys.wallets.listOverviews);
      toast.success(t('wallet_list.delete_success'));
      confirm.onFalse();
    } catch (error) {
      handleActionError(error);
    } finally {
      isDeleting.onFalse();
    }
  };

  return (
    <>
      <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
        <Iconify icon="eva:more-vertical-fill" />
      </IconButton>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        <MenuList>
          <MenuItem
            component={RouterLink}
            href={paths.admin.wallet(wallet.id)}
            onClick={popover.onClose}
          >
            <Iconify icon="solar:eye-bold" />
            {t('view')}
          </MenuItem>
          <MenuItem
            component={RouterLink}
            href={paths.admin.account(wallet.account_id)}
            onClick={popover.onClose}
          >
            <Iconify icon="solar:users-group-two-rounded-bold-duotone" />
            {t('accounts_view.account')}
          </MenuItem>

          <CopyMenuItem value={wallet.id} title={t('wallets_view.copy_wallet_id')} />

          <RoleBasedGuard permissions={[Permission.WRITE_WALLET]}>
            <Divider sx={{ borderStyle: 'dashed' }} />
            <MenuItem
              onClick={() => {
                confirm.onTrue();
                popover.onClose();
              }}
              sx={{ color: 'error.main' }}
            >
              <Iconify icon="solar:trash-bin-trash-bold" />
              {t('delete')}
            </MenuItem>
          </RoleBasedGuard>
        </MenuList>
      </CustomPopover>

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('delete')}
        content={t('confirm_delete')}
        action={
          <Button
            color="error"
            variant="contained"
            loading={isDeleting.value}
            onClick={handleDelete}
          >
            {t('delete')}
          </Button>
        }
      />
    </>
  );
}

// ----------------------------------------------------------------------

export function WalletsView() {
  const searchParams = useSearchParams();
  const walletId = searchParams.get('id');

  return walletId ? <WalletDetailsView id={walletId} /> : <WalletsDirectoryView />;
}

function WalletsDirectoryView() {
  const { t } = useTranslate();
  const theme = useTheme();
  const table = useTable({ defaultRowsPerPage: 25 });
  const newWallet = useBoolean();
  const { walletOverviews, walletOverviewsLoading, walletOverviewsError } =
    useListWalletOverviews();
  const [query, setQuery] = useState('');
  const [readinessFilter, setReadinessFilter] = useState<WalletReadiness>('all');
  const [sort, setSort] = useState<WalletSort>('activity');

  const errors = [walletOverviewsError];
  const data = [walletOverviews];
  const isLoading = [walletOverviewsLoading];
  const failed = shouldFail(errors, data, isLoading);
  const wallets = useMemo(() => walletOverviews ?? [], [walletOverviews]);
  const filteredWallets = useMemo(
    () =>
      sortWallets(
        wallets.filter(
          (wallet) =>
            (readinessFilter === 'all' || walletReadiness(wallet) === readinessFilter) &&
            walletMatchesSearch(wallet, query)
        ),
        sort
      ),
    [query, readinessFilter, sort, wallets]
  );
  const walletsInPage = filteredWallets.slice(
    table.page * table.rowsPerPage,
    table.page * table.rowsPerPage + table.rowsPerPage
  );
  const denseHeight = table.dense ? 56 : 76;
  const statusTabs: Array<{
    value: WalletReadiness;
    label: string;
    color: LabelColor;
    icon: string;
    analyticColor: string;
  }> = [
    {
      value: 'all',
      label: t('wallets_view.all'),
      color: 'default',
      icon: 'solar:safe-square-bold-duotone',
      analyticColor: theme.palette.info.main,
    },
    {
      value: 'ready',
      label: t('wallets_view.ready'),
      color: 'success',
      icon: 'solar:check-circle-bold-duotone',
      analyticColor: theme.palette.success.main,
    },
    {
      value: 'paused',
      label: t('wallets_view.paused'),
      color: 'warning',
      icon: 'solar:pause-circle-bold-duotone',
      analyticColor: theme.palette.warning.main,
    },
    {
      value: 'missing',
      label: t('wallets_view.missing'),
      color: 'default',
      icon: 'solar:minus-circle-bold-duotone',
      analyticColor: theme.palette.text.disabled,
    },
  ];
  const countByReadiness = (readiness: WalletReadiness) =>
    readiness === 'all'
      ? wallets.length
      : wallets.filter((wallet) => walletReadiness(wallet) === readiness).length;
  const percentageByReadiness = (readiness: WalletReadiness) =>
    wallets.length ? (countByReadiness(readiness) / wallets.length) * 100 : 0;

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_WALLET]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('wallets_directory')}
              links={[{ name: t('accounts') }, { name: t('wallets_directory') }]}
              action={
                <RoleBasedGuard permissions={[Permission.WRITE_WALLET]}>
                  <Button
                    variant="contained"
                    startIcon={<Iconify icon="mingcute:add-line" />}
                    onClick={newWallet.onTrue}
                  >
                    {t('wallets_view.new_wallet')}
                  </Button>
                </RoleBasedGuard>
              }
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Card sx={{ mb: 3 }}>
              <Scrollbar>
                <Stack
                  direction="row"
                  divider={
                    <Divider orientation="vertical" flexItem sx={{ borderStyle: 'dashed' }} />
                  }
                  sx={{ py: 2 }}
                >
                  {statusTabs.map((tab) => (
                    <ItemAnalytic
                      key={tab.value}
                      title={tab.label}
                      total={countByReadiness(tab.value)}
                      percent={percentageByReadiness(tab.value)}
                      icon={tab.icon}
                      color={tab.analyticColor}
                      countSuffix={t('wallets')}
                    />
                  ))}
                </Stack>
              </Scrollbar>
            </Card>

            <Card>
              <Tabs
                value={readinessFilter}
                onChange={(_, value: WalletReadiness) => {
                  setReadinessFilter(value);
                  table.onResetPage();
                }}
                sx={{
                  px: 2.5,
                  boxShadow: `inset 0 -2px 0 0 ${alpha(theme.palette.grey[500], 0.08)}`,
                }}
              >
                {statusTabs.map((tab) => (
                  <Tab
                    key={tab.value}
                    value={tab.value}
                    label={tab.label}
                    iconPosition="end"
                    icon={
                      <Label
                        variant={
                          tab.value === 'all' || tab.value === readinessFilter ? 'filled' : 'soft'
                        }
                        color={tab.color}
                      >
                        {countByReadiness(tab.value)}
                      </Label>
                    }
                  />
                ))}
              </Tabs>

              <Stack
                direction={{ xs: 'column', md: 'row' }}
                spacing={1.5}
                sx={{ p: 2.5, alignItems: { md: 'center' } }}
              >
                <TextField
                  fullWidth
                  value={query}
                  placeholder={t('wallets_view.search_placeholder')}
                  onChange={(event) => {
                    setQuery(event.target.value);
                    table.onResetPage();
                  }}
                  slotProps={{
                    input: {
                      startAdornment: (
                        <InputAdornment position="start">
                          <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled' }} />
                        </InputAdornment>
                      ),
                    },
                  }}
                />
                <TextField
                  select
                  label={t('wallets_view.sort_by')}
                  value={sort}
                  onChange={(event) => setSort(event.target.value as WalletSort)}
                  sx={{ minWidth: { md: 220 } }}
                >
                  <MenuItem value="activity">{t('wallets_view.sort_activity')}</MenuItem>
                  <MenuItem value="recent">{t('wallets_view.sort_recent')}</MenuItem>
                  <MenuItem value="network">{t('wallets_view.sort_network')}</MenuItem>
                  <MenuItem value="account">{t('wallets_view.sort_account')}</MenuItem>
                </TextField>
              </Stack>

              <TableContainer sx={{ position: 'relative', overflow: 'unset' }}>
                <Scrollbar>
                  <Table size={table.dense ? 'small' : 'medium'} sx={{ minWidth: 1180 }}>
                    <TableHeadCustom
                      headCells={tableHead(t)}
                      rowCount={filteredWallets.length}
                      numSelected={table.selected.length}
                      onSelectAllRows={(checked) =>
                        table.onSelectAllRows(
                          checked,
                          filteredWallets.map((wallet) => wallet.id)
                        )
                      }
                    />
                    <TableBody>
                      {walletsInPage.map((wallet) => {
                        const readiness = walletReadiness(wallet);
                        const activity = wallet.n_invoices + wallet.n_payments;

                        return (
                          <TableRow
                            key={wallet.id}
                            hover
                            selected={table.selected.includes(wallet.id)}
                          >
                            <TableCell padding="checkbox">
                              <Checkbox
                                checked={table.selected.includes(wallet.id)}
                                onClick={() => table.onSelectRow(wallet.id)}
                              />
                            </TableCell>
                            <TableCell>
                              <Stack spacing={0.25} sx={{ minWidth: 180 }}>
                                <Typography variant="subtitle2" noWrap>
                                  {walletName(wallet)}
                                </Typography>
                                <Typography variant="caption" color="text.secondary" noWrap>
                                  {compactId(wallet.id)}
                                </Typography>
                              </Stack>
                            </TableCell>
                            <TableCell>
                              <Link
                                component={RouterLink}
                                href={paths.admin.account(wallet.account_id)}
                                color="inherit"
                                underline="hover"
                                sx={{
                                  maxWidth: 1,
                                  display: 'block',
                                  overflow: 'hidden',
                                  whiteSpace: 'nowrap',
                                  fontFamily: 'monospace',
                                  textOverflow: 'ellipsis',
                                  typography: 'body2',
                                }}
                              >
                                {compactId(wallet.account_id)}
                              </Link>
                            </TableCell>
                            <TableCell>
                              <Stack spacing={0.25}>
                                <Typography variant="subtitle2">
                                  {wallet.asset?.display_ticker ?? t('wallets_view.unknown_asset')}
                                </Typography>
                                <Typography variant="caption" color="text.secondary">
                                  {wallet.asset?.code ?? '-'} · {wallet.asset?.network ?? '-'}
                                </Typography>
                              </Stack>
                            </TableCell>
                            <TableCell align="right">
                              <SatsWithIcon
                                amountMSats={wallet.balance.available_msat}
                                variant="body2"
                              />
                            </TableCell>
                            <TableCell>
                              <Stack spacing={0.5} sx={{ alignItems: 'flex-start' }}>
                                <Label color={readinessColor(readiness)}>
                                  {t(`wallets_view.${readiness}`)}
                                </Label>
                                {wallet.ln_address?.username && (
                                  <Typography variant="caption" color="text.secondary" noWrap>
                                    {displayLnAddress(wallet.ln_address.username)}
                                  </Typography>
                                )}
                              </Stack>
                            </TableCell>
                            <TableCell>
                              <Link
                                component={RouterLink}
                                href={paths.admin.walletTransactions(wallet.id)}
                                color="inherit"
                                underline="hover"
                                sx={{ typography: 'body2' }}
                              >
                                {t('wallets_view.activity_count', { count: activity })}
                              </Link>
                            </TableCell>
                            <TableCell>{fDate(wallet.created_at)}</TableCell>
                            <TableCell align="right" sx={{ px: 1 }}>
                              <WalletActions wallet={wallet} />
                            </TableCell>
                          </TableRow>
                        );
                      })}

                      <TableEmptyRows
                        height={denseHeight}
                        emptyRows={emptyRows(table.page, table.rowsPerPage, filteredWallets.length)}
                      />
                      <TableNoData notFound={!filteredWallets.length} />
                    </TableBody>
                  </Table>
                </Scrollbar>
              </TableContainer>

              <TablePaginationCustom
                count={filteredWallets.length}
                page={table.page}
                rowsPerPage={table.rowsPerPage}
                onPageChange={table.onChangePage}
                onRowsPerPageChange={table.onChangeRowsPerPage}
                dense={table.dense}
                onChangeDense={table.onChangeDense}
              />
            </Card>

            <RegisterWalletDrawer
              open={newWallet.value}
              onClose={newWallet.onFalse}
              onSuccess={() => {
                mutate(endpointKeys.wallets.listOverviews);
                newWallet.onFalse();
              }}
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
