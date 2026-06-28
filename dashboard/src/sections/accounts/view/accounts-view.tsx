'use client';

import type { ReactNode } from 'react';
import type { WalletOverview } from 'src/lib/swissknife';

import { sumBy } from 'es-toolkit';
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
import { displayLnAddress } from 'src/utils/lnurl';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletOverviews } from 'src/actions/wallet';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

type ReadinessFilter = 'all' | 'ready' | 'paused' | 'missing';
type AccountSort = 'activity' | 'balance' | 'recent' | 'identity';
type IconBoxColor = 'primary' | 'info' | 'warning' | 'success';

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
      sx={[
        (theme) => ({
          width: size,
          height: size,
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
      <Iconify icon={icon} width={iconSize} />
    </Box>
  );
}

function accountName(account: WalletOverview, index: number) {
  if (account.ln_address?.username) {
    return displayLnAddress(account.ln_address.username);
  }

  return `Account ${index + 1}`;
}

function accountReadiness(account: WalletOverview) {
  if (account.ln_address?.active) return 'ready';
  if (account.ln_address) return 'paused';
  return 'missing';
}

export function AccountsView() {
  const { t } = useTranslate();
  const { walletOverviews, walletOverviewsLoading, walletOverviewsError } =
    useListWalletOverviews();
  const [query, setQuery] = useState('');
  const [readinessFilter, setReadinessFilter] = useState<ReadinessFilter>('all');
  const [sort, setSort] = useState<AccountSort>('activity');

  const errors = [walletOverviewsError];
  const data = [walletOverviews];
  const isLoading = [walletOverviewsLoading];
  const failed = shouldFail(errors, data, isLoading);

  const accounts = useMemo(() => walletOverviews ?? [], [walletOverviews]);
  const totalBalance = sumBy(accounts, (account) => account.balance.available_msat || 0);
  const identityReady = accounts.filter((account) => accountReadiness(account) === 'ready').length;
  const totalInvoices = sumBy(accounts, (account) => account.n_invoices);
  const totalPayments = sumBy(accounts, (account) => account.n_payments);
  const filteredAccounts = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();

    return accounts
      .filter((account) => {
        if (readinessFilter !== 'all' && accountReadiness(account) !== readinessFilter) {
          return false;
        }

        if (!normalizedQuery) return true;

        const lnAddress = account.ln_address?.username
          ? displayLnAddress(account.ln_address.username)
          : '';
        const haystack = [account.id, account.user_id, lnAddress].join(' ').toLowerCase();

        return haystack.includes(normalizedQuery);
      })
      .sort((left, right) => {
        if (sort === 'balance') {
          return (right.balance.available_msat || 0) - (left.balance.available_msat || 0);
        }

        if (sort === 'recent') {
          return new Date(right.created_at).getTime() - new Date(left.created_at).getTime();
        }

        if (sort === 'identity') {
          return accountReadiness(left).localeCompare(accountReadiness(right));
        }

        return (
          right.n_invoices +
          right.n_payments +
          right.n_contacts -
          (left.n_invoices + left.n_payments + left.n_contacts)
        );
      });
  }, [accounts, query, readinessFilter, sort]);

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_WALLET]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('accounts_directory')}
              links={[{ name: t('accounts') }, { name: t('accounts_directory') }]}
              action={
                <Button
                  href={paths.admin.wallets}
                  color="inherit"
                  variant="outlined"
                  startIcon={<Iconify icon="solar:safe-square-bold-duotone" />}
                >
                  {t('accounts_view.admin_tools')}
                </Button>
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
                        <Typography variant="h4">{t('accounts_view.hero_title')}</Typography>
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
                          icon="solar:wallet-money-bold-duotone"
                          label={t('wallet_list.balance')}
                          value={<SatsWithIcon amountMSats={totalBalance} variant="h6" />}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="solar:fingerprint-bold-duotone"
                          label={t('accounts_view.identity_ready')}
                          value={`${identityReady}/${accounts.length}`}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="eva:diagonal-arrow-left-down-fill"
                          label={t('wallet_list.invoices')}
                          value={totalInvoices.toLocaleString()}
                          color="success"
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="eva:diagonal-arrow-right-up-fill"
                          label={t('wallet_list.payments')}
                          value={totalPayments.toLocaleString()}
                          color="warning"
                        />
                      </Grid>
                    </Grid>
                  </Grid>
                </Grid>
              </Card>

              {accounts.length ? (
                <DirectoryControls
                  query={query}
                  sort={sort}
                  total={accounts.length}
                  visible={filteredAccounts.length}
                  readinessFilter={readinessFilter}
                  onQueryChange={setQuery}
                  onSortChange={setSort}
                  onReadinessChange={setReadinessFilter}
                />
              ) : null}

              {filteredAccounts.length ? (
                <Grid container spacing={2.5}>
                  {filteredAccounts.map((account, index) => (
                    <Grid key={account.id} size={{ xs: 12, md: 6, lg: 4 }}>
                      <AccountCard account={account} index={index} />
                    </Grid>
                  ))}
                </Grid>
              ) : (
                <EmptyContent
                  title={
                    accounts.length
                      ? t('accounts_view.no_results_title')
                      : t('accounts_view.empty_title')
                  }
                  description={
                    accounts.length
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
  readinessFilter,
  onQueryChange,
  onSortChange,
  onReadinessChange,
}: {
  query: string;
  sort: AccountSort;
  total: number;
  visible: number;
  readinessFilter: ReadinessFilter;
  onQueryChange: (value: string) => void;
  onSortChange: (value: AccountSort) => void;
  onReadinessChange: (value: ReadinessFilter) => void;
}) {
  const { t } = useTranslate();

  return (
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
            sx={{ minWidth: { md: 200 } }}
          >
            <MenuItem value="activity">{t('accounts_view.sort_activity')}</MenuItem>
            <MenuItem value="balance">{t('accounts_view.sort_balance')}</MenuItem>
            <MenuItem value="recent">{t('accounts_view.sort_recent')}</MenuItem>
            <MenuItem value="identity">{t('accounts_view.sort_identity')}</MenuItem>
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
            value={readinessFilter}
            onChange={(_, value: ReadinessFilter | null) => {
              if (value) onReadinessChange(value);
            }}
          >
            <ToggleButton value="all">{t('accounts_view.all')}</ToggleButton>
            <ToggleButton value="ready">{t('accounts_view.ready')}</ToggleButton>
            <ToggleButton value="paused">{t('accounts_view.paused')}</ToggleButton>
            <ToggleButton value="missing">{t('accounts_view.missing')}</ToggleButton>
          </ToggleButtonGroup>

          <Label color={visible === total ? 'default' : 'info'}>
            {t('accounts_view.visible_count', { visible, total })}
          </Label>
        </Stack>
      </Stack>
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
      sx={[
        (theme) => ({
          p: 2,
          height: 1,
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
        }),
      ]}
    >
      <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
        <OutlinedIconBox icon={icon} color={color} size={44} iconSize={26} />
        <Stack sx={{ minWidth: 0 }}>
          <Typography variant="caption" color="text.secondary">
            {label}
          </Typography>
          {typeof value === 'string' ? <Typography variant="h6">{value}</Typography> : value}
        </Stack>
      </Stack>
    </Box>
  );
}

function AccountCard({ account, index }: { account: WalletOverview; index: number }) {
  const { t } = useTranslate();
  const readiness = accountReadiness(account);

  return (
    <Card sx={{ p: 2.5, borderRadius: 1, height: 1 }}>
      <Stack spacing={2.5}>
        <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
          <OutlinedIconBox
            icon="solar:user-rounded-bold-duotone"
            color="warning"
            size={44}
            iconSize={26}
          />

          <Stack sx={{ minWidth: 0, flex: 1 }}>
            <Typography variant="subtitle1" noWrap>
              {accountName(account, index)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {t('accounts_view.created_on', { date: fDate(account.created_at) })}
            </Typography>
          </Stack>

          <Label
            color={
              (readiness === 'ready' && 'success') ||
              (readiness === 'paused' && 'warning') ||
              'default'
            }
          >
            {t(`accounts_view.${readiness}`)}
          </Label>
        </Stack>

        <Stack spacing={0.5}>
          <Typography variant="caption" color="text.secondary">
            {t('wallet_list.balance')}
          </Typography>
          <SatsWithIcon amountMSats={account.balance.available_msat || 0} variant="h5" />
        </Stack>

        <Divider sx={{ borderStyle: 'dashed' }} />

        <Stack spacing={1}>
          <InfoRow
            label={t('wallet_list.user')}
            value={account.user_id}
            copyValue={account.user_id}
          />
          <InfoRow label={t('wallet')} value={account.id} copyValue={account.id} />
          <InfoRow
            label={t('wallet_list.ln_address')}
            value={
              account.ln_address?.username
                ? displayLnAddress(account.ln_address.username)
                : t('accounts_view.missing')
            }
            copyValue={
              account.ln_address?.username ? displayLnAddress(account.ln_address.username) : ''
            }
            href={account.ln_address?.id ? paths.admin.lnAddress(account.ln_address.id) : undefined}
          />
        </Stack>

        <Divider sx={{ borderStyle: 'dashed' }} />

        <Grid container spacing={1.5}>
          <Grid size={4}>
            <MiniStat label={t('wallet_list.invoices')} value={account.n_invoices} />
          </Grid>
          <Grid size={4}>
            <MiniStat label={t('wallet_list.payments')} value={account.n_payments} />
          </Grid>
          <Grid size={4}>
            <MiniStat label={t('wallet_list.contacts')} value={account.n_contacts} />
          </Grid>
        </Grid>

        <Button
          href={`${paths.admin.wallets}?id=${account.id}`}
          color="inherit"
          variant="soft"
          endIcon={<Iconify icon="eva:arrow-ios-forward-fill" />}
        >
          {t('accounts_view.manage_in_admin')}
        </Button>
      </Stack>
    </Card>
  );
}

function InfoRow({
  href,
  label,
  value,
  copyValue,
}: {
  href?: string;
  label: string;
  value: string;
  copyValue?: string;
}) {
  const content = (
    <Typography
      noWrap
      variant="body2"
      component={href ? 'a' : 'span'}
      href={href}
      sx={{
        color: href ? 'primary.main' : 'text.primary',
        textDecoration: 'none',
        '&:hover': { textDecoration: href ? 'underline' : 'none' },
      }}
    >
      {truncateText(value, 34)}
    </Typography>
  );

  return (
    <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
      <Typography variant="caption" color="text.secondary" sx={{ width: 88, flexShrink: 0 }}>
        {label}
      </Typography>
      <Box sx={{ minWidth: 0, flex: 1 }}>{content}</Box>
      {copyValue && <CopyButton value={copyValue} title="Copy" />}
    </Stack>
  );
}

function MiniStat({ label, value }: { label: string; value: number }) {
  return (
    <Stack spacing={0.25}>
      <Typography variant="subtitle2">{value.toLocaleString()}</Typography>
      <Typography variant="caption" color="text.secondary" noWrap>
        {label}
      </Typography>
    </Stack>
  );
}
