'use client';

import type { ReactNode } from 'react';
import type { Wallet, Account } from 'src/lib/swissknife';

import { useMemo, useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import ToggleButton from '@mui/material/ToggleButton';
import InputAdornment from '@mui/material/InputAdornment';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { paths } from 'src/routes/paths';

import { fDate } from 'src/utils/format-time';
import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useListAccounts } from 'src/actions/account';
import { DashboardContent } from 'src/layouts/dashboard';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import { useTable, TablePaginationCustom } from 'src/components/table';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

type IdentityFilter = 'all' | 'linked' | 'unlinked';
type AccountSort = 'wallets' | 'recent' | 'name';
type IconBoxColor = 'primary' | 'info' | 'warning' | 'success';

export function accountMatchesSearch(account: Account, wallets: Wallet[], query: string) {
  const normalizedQuery = query.trim().toLowerCase();

  if (!normalizedQuery) return true;

  return [
    account.id,
    account.display_name,
    account.identity?.provider,
    account.identity?.subject,
    ...wallets.flatMap((wallet) => [
      wallet.id,
      wallet.label,
      wallet.asset?.code,
      wallet.asset?.display_ticker,
      wallet.asset?.network,
    ]),
  ]
    .filter(Boolean)
    .some((value) => value!.toLowerCase().includes(normalizedQuery));
}

function accountName(account: Account) {
  return account.display_name || account.identity?.subject || account.id;
}

function compactId(id: string) {
  return id.length > 20 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;
}

function sortAccounts(accounts: Account[], sort: AccountSort) {
  return [...accounts].sort((left, right) => {
    if (sort === 'recent') {
      return new Date(right.created_at).getTime() - new Date(left.created_at).getTime();
    }

    if (sort === 'name') return accountName(left).localeCompare(accountName(right));

    return right.wallets.length - left.wallets.length;
  });
}

// ----------------------------------------------------------------------

export function AccountDirectory({ onCreate }: { onCreate: VoidFunction }) {
  const { t } = useTranslate();
  const table = useTable({ defaultRowsPerPage: 12 });
  const { accounts, accountsLoading, accountsError } = useListAccounts();
  const [query, setQuery] = useState('');
  const [identityFilter, setIdentityFilter] = useState<IdentityFilter>('all');
  const [sort, setSort] = useState<AccountSort>('recent');

  const errors = [accountsError];
  const data = [accounts];
  const isLoading = [accountsLoading];
  const failed = shouldFail(errors, data, isLoading);
  const accountList = useMemo(() => accounts ?? [], [accounts]);
  const filteredAccounts = useMemo(
    () =>
      sortAccounts(
        accountList.filter(
          (account) =>
            (identityFilter === 'all' ||
              (identityFilter === 'linked' ? !!account.identity : !account.identity)) &&
            accountMatchesSearch(account, account.wallets, query)
        ),
        sort
      ),
    [accountList, identityFilter, query, sort]
  );
  const accountsInPage = filteredAccounts.slice(
    table.page * table.rowsPerPage,
    table.page * table.rowsPerPage + table.rowsPerPage
  );
  const walletCount = accountList.reduce((total, account) => total + account.wallets.length, 0);
  const identityCount = accountList.filter((account) => account.identity).length;
  const networkCount = new Set(
    accountList
      .flatMap((account) => account.wallets.map((wallet) => wallet.asset?.network))
      .filter(Boolean)
  ).size;

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_ACCOUNT]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('accounts_directory')}
              links={[{ name: t('accounts') }, { name: t('accounts_directory') }]}
              action={
                <RoleBasedGuard permissions={[Permission.WRITE_ACCOUNT]}>
                  <Button
                    variant="contained"
                    startIcon={<Iconify icon="mingcute:add-line" />}
                    onClick={onCreate}
                  >
                    {t('accounts_view.new_account')}
                  </Button>
                </RoleBasedGuard>
              }
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Stack spacing={3}>
              <Card sx={{ p: 3, borderRadius: 1 }}>
                <Grid container spacing={3} sx={{ alignItems: 'stretch' }}>
                  <Grid size={{ xs: 12, md: 5 }}>
                    <Stack spacing={2} sx={{ height: 1, justifyContent: 'center' }}>
                      <OutlinedIconBox
                        icon="solar:users-group-two-rounded-bold-duotone"
                        color="info"
                        size={56}
                        iconSize={32}
                      />
                      <Stack spacing={1}>
                        <Typography variant="h5">{t('accounts_view.hero_title')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('accounts_view.hero_subheader')}
                        </Typography>
                      </Stack>
                    </Stack>
                  </Grid>

                  <Grid size={{ xs: 12, md: 7 }}>
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="solar:users-group-two-rounded-bold-duotone"
                          label={t('accounts')}
                          value={accountList.length.toLocaleString()}
                          color="info"
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="solar:shield-user-bold-duotone"
                          label={t('accounts_view.linked_identities')}
                          value={identityCount.toLocaleString()}
                          color="success"
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="solar:safe-square-bold-duotone"
                          label={t('wallets')}
                          value={walletCount.toLocaleString()}
                          color="warning"
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="solar:global-bold-duotone"
                          label={t('wallets_view.networks')}
                          value={networkCount.toLocaleString()}
                        />
                      </Grid>
                    </Grid>
                  </Grid>
                </Grid>
              </Card>

              {accountList.length > 0 && (
                <DirectoryControls
                  query={query}
                  sort={sort}
                  total={accountList.length}
                  visible={filteredAccounts.length}
                  identityFilter={identityFilter}
                  onQueryChange={(value) => {
                    table.onResetPage();
                    setQuery(value);
                  }}
                  onSortChange={(value) => {
                    table.onResetPage();
                    setSort(value);
                  }}
                  onIdentityFilterChange={(value) => {
                    table.onResetPage();
                    setIdentityFilter(value);
                  }}
                />
              )}

              {filteredAccounts.length ? (
                <Stack spacing={1}>
                  <Grid container spacing={2.5}>
                    {accountsInPage.map((account) => (
                      <Grid key={account.id} size={{ xs: 12, md: 6, lg: 4 }}>
                        <AccountCard account={account} />
                      </Grid>
                    ))}
                  </Grid>

                  <TablePaginationCustom
                    count={filteredAccounts.length}
                    page={table.page}
                    rowsPerPage={table.rowsPerPage}
                    rowsPerPageOptions={[12, 24, 48]}
                    onPageChange={table.onChangePage}
                    onRowsPerPageChange={table.onChangeRowsPerPage}
                  />
                </Stack>
              ) : (
                <EmptyContent
                  title={
                    accountList.length
                      ? t('accounts_view.no_results_title')
                      : t('accounts_view.empty_title')
                  }
                  description={
                    accountList.length
                      ? t('accounts_view.no_results_description')
                      : t('accounts_view.empty_description')
                  }
                  sx={{ py: 8 }}
                />
              )}
            </Stack>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}

function DirectoryControls({
  query,
  sort,
  total,
  visible,
  identityFilter,
  onQueryChange,
  onSortChange,
  onIdentityFilterChange,
}: {
  query: string;
  sort: AccountSort;
  total: number;
  visible: number;
  identityFilter: IdentityFilter;
  onQueryChange: (value: string) => void;
  onSortChange: (value: AccountSort) => void;
  onIdentityFilterChange: (value: IdentityFilter) => void;
}) {
  const { t } = useTranslate();

  return (
    <Box
      sx={(theme) => ({
        p: 2,
        borderRadius: 1,
        bgcolor: 'background.neutral',
        border: `1px solid ${theme.vars.palette.divider}`,
      })}
    >
      <Stack spacing={2}>
        <Stack
          direction={{ xs: 'column', md: 'row' }}
          spacing={1.5}
          sx={{ alignItems: { xs: 'stretch', md: 'center' } }}
        >
          <TextField
            fullWidth
            value={query}
            placeholder={t('accounts_view.search_placeholder')}
            onChange={(event) => onQueryChange(event.target.value)}
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
            label={t('accounts_view.sort_by')}
            value={sort}
            onChange={(event) => onSortChange(event.target.value as AccountSort)}
            sx={{ minWidth: { md: 220 } }}
          >
            <MenuItem value="wallets">{t('accounts_view.sort_wallets')}</MenuItem>
            <MenuItem value="recent">{t('accounts_view.sort_recent')}</MenuItem>
            <MenuItem value="name">{t('accounts_view.sort_name')}</MenuItem>
          </TextField>
        </Stack>

        <Stack
          direction={{ xs: 'column', md: 'row' }}
          spacing={1.5}
          sx={{ alignItems: { xs: 'stretch', md: 'center' }, justifyContent: 'space-between' }}
        >
          <ToggleButtonGroup
            exclusive
            size="small"
            value={identityFilter}
            onChange={(_, value: IdentityFilter | null) => value && onIdentityFilterChange(value)}
          >
            <ToggleButton value="all">{t('wallets_view.all')}</ToggleButton>
            <ToggleButton value="linked">{t('accounts_view.identity_linked')}</ToggleButton>
            <ToggleButton value="unlinked">{t('accounts_view.no_identity')}</ToggleButton>
          </ToggleButtonGroup>
          <Label color={visible === total ? 'default' : 'info'}>
            {t('accounts_view.visible_count', { visible, total })}
          </Label>
        </Stack>
      </Stack>
    </Box>
  );
}

function AccountCard({ account }: { account: Account }) {
  const { t } = useTranslate();
  const wallets = account.wallets.slice(0, 3);
  const remainingWallets = account.wallets.length - wallets.length;

  return (
    <Card sx={{ p: 2.5, borderRadius: 1, height: 1 }}>
      <Stack spacing={2.25} sx={{ height: 1 }}>
        <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
          <OutlinedIconBox
            icon="solar:users-group-two-rounded-bold-duotone"
            color="info"
            size={44}
            iconSize={26}
          />
          <Stack sx={{ minWidth: 0, flex: 1 }}>
            <Typography variant="subtitle1" noWrap>
              {accountName(account)}
            </Typography>
            <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', minWidth: 0 }}>
              <Typography variant="caption" color="text.secondary" noWrap>
                {compactId(account.id)}
              </Typography>
              <CopyButton value={account.id} title={t('copy')} />
            </Stack>
          </Stack>
          <Label color={account.identity ? 'success' : 'default'}>
            {account.identity?.provider.toUpperCase() ?? t('accounts_view.no_identity')}
          </Label>
        </Stack>

        <Stack spacing={0.5}>
          <Typography variant="caption" color="text.secondary">
            {t('accounts_view.identity')}
          </Typography>
          <Typography variant="body2" noWrap>
            {account.identity?.subject ?? t('accounts_view.no_identity')}
          </Typography>
        </Stack>

        <Divider sx={{ borderStyle: 'dashed' }} />

        <Stack spacing={1.25}>
          <Typography variant="subtitle2">
            {t('accounts_view.wallets_title', { count: account.wallets.length })}
          </Typography>

          {wallets.length ? (
            wallets.map((wallet) => (
              <Box
                key={wallet.id}
                component="a"
                href={paths.admin.wallet(wallet.id)}
                sx={(theme) => ({
                  p: 1.25,
                  gap: 1,
                  display: 'flex',
                  color: 'inherit',
                  borderRadius: 1,
                  alignItems: 'center',
                  textDecoration: 'none',
                  bgcolor: 'background.neutral',
                  border: `1px solid ${theme.vars.palette.divider}`,
                  '&:hover': { bgcolor: 'action.hover' },
                })}
              >
                <Stack sx={{ minWidth: 0, flex: 1 }}>
                  <Typography variant="subtitle2" noWrap>
                    {wallet.label || wallet.asset?.display_ticker || compactId(wallet.id)}
                  </Typography>
                  <Typography variant="caption" color="text.secondary" noWrap>
                    {wallet.asset?.code ?? '-'} · {wallet.asset?.network ?? '-'}
                  </Typography>
                </Stack>
                <SatsWithIcon amountMSats={wallet.balance.available_msat} variant="body2" />
                <Iconify icon="eva:arrow-ios-forward-fill" sx={{ color: 'text.disabled' }} />
              </Box>
            ))
          ) : (
            <Typography variant="body2" color="text.secondary">
              {t('accounts_view.no_wallets')}
            </Typography>
          )}

          {remainingWallets > 0 && (
            <Typography variant="caption" color="text.secondary">
              {t('accounts_view.more_wallets', { count: remainingWallets })}
            </Typography>
          )}
        </Stack>

        <Divider sx={{ borderStyle: 'dashed' }} />

        <Grid container spacing={1.5}>
          <Grid size={4}>
            <MiniStat label={t('wallets')} value={account.wallets.length.toLocaleString()} />
          </Grid>
          <Grid size={4}>
            <MiniStat
              label={t('accounts_view.permissions')}
              value={(account.permissions?.length ?? 0).toLocaleString()}
            />
          </Grid>
          <Grid size={4}>
            <MiniStat label={t('wallet_list.created')} value={fDate(account.created_at)} />
          </Grid>
        </Grid>

        <Button
          href={paths.admin.account(account.id)}
          color="inherit"
          variant="soft"
          endIcon={<Iconify icon="eva:arrow-ios-forward-fill" />}
          sx={{ mt: 'auto' }}
        >
          {t('details')}
        </Button>
      </Stack>
    </Card>
  );
}

function OutlinedIconBox({
  icon,
  color,
  size,
  iconSize,
}: {
  icon: string;
  color: IconBoxColor;
  size: number;
  iconSize: number;
}) {
  return (
    <Box
      sx={(theme) => ({
        width: size,
        height: size,
        display: 'grid',
        flexShrink: 0,
        borderRadius: 1,
        placeItems: 'center',
        color: theme.vars.palette[color].main,
        bgcolor: 'background.paper',
        border: `1px solid ${theme.vars.palette[color].main}`,
      })}
    >
      <Iconify icon={icon} width={iconSize} />
    </Box>
  );
}

function SummaryTile({
  icon,
  label,
  value,
  color = 'primary',
}: {
  icon: string;
  label: string;
  value: ReactNode;
  color?: IconBoxColor;
}) {
  return (
    <Box
      sx={(theme) => ({
        p: 2,
        height: 1,
        borderRadius: 1,
        bgcolor: 'background.neutral',
        border: `1px solid ${theme.vars.palette.divider}`,
      })}
    >
      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
        <OutlinedIconBox icon={icon} color={color} size={40} iconSize={22} />
        <Stack spacing={0.25}>
          <Typography variant="h5">{value}</Typography>
          <Typography variant="caption" color="text.secondary">
            {label}
          </Typography>
        </Stack>
      </Stack>
    </Box>
  );
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <Stack spacing={0.25} sx={{ alignItems: 'center', textAlign: 'center' }}>
      <Typography variant="subtitle2">{value}</Typography>
      <Typography variant="caption" color="text.secondary" noWrap>
        {label}
      </Typography>
    </Stack>
  );
}
