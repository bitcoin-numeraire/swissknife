'use client';

import type { ReactNode } from 'react';
import type { WalletOverview } from 'src/lib/swissknife';

import { sumBy } from 'es-toolkit';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';

import { paths } from 'src/routes/paths';

import { fDate } from 'src/utils/format-time';
import { shouldFail } from 'src/utils/errors';
import { displayLnAddress } from 'src/utils/lnurl';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletOverviews } from 'src/actions/wallet';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

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

  const errors = [walletOverviewsError];
  const data = [walletOverviews];
  const isLoading = [walletOverviewsLoading];
  const failed = shouldFail(errors, data, isLoading);

  const accounts = walletOverviews ?? [];
  const totalBalance = sumBy(accounts, (account) => account.balance.available_msat || 0);
  const identityReady = accounts.filter((account) => accountReadiness(account) === 'ready').length;
  const totalInvoices = sumBy(accounts, (account) => account.n_invoices);
  const totalPayments = sumBy(accounts, (account) => account.n_payments);

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
                      <Box
                        sx={{
                          width: 56,
                          height: 56,
                          display: 'grid',
                          borderRadius: 1,
                          placeItems: 'center',
                          color: 'info.main',
                          bgcolor: 'info.lighter',
                        }}
                      >
                        <Iconify icon="solar:users-group-two-rounded-bold-duotone" width={32} />
                      </Box>
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
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <SummaryTile
                          icon="eva:diagonal-arrow-right-up-fill"
                          label={t('wallet_list.payments')}
                          value={totalPayments.toLocaleString()}
                        />
                      </Grid>
                    </Grid>
                  </Grid>
                </Grid>
              </Card>

              {accounts.length ? (
                <Grid container spacing={2.5}>
                  {accounts.map((account, index) => (
                    <Grid key={account.id} size={{ xs: 12, md: 6, lg: 4 }}>
                      <AccountCard account={account} index={index} />
                    </Grid>
                  ))}
                </Grid>
              ) : (
                <EmptyContent
                  title={t('accounts_view.empty_title')}
                  description={t('accounts_view.empty_description')}
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

function SummaryTile({
  icon,
  label,
  value,
}: {
  icon: string;
  label: string;
  value: ReactNode;
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
        <Iconify icon={icon} width={28} sx={{ color: 'primary.main' }} />
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
          <Box
            sx={{
              width: 44,
              height: 44,
              display: 'grid',
              borderRadius: 1,
              flexShrink: 0,
              placeItems: 'center',
              color: 'warning.main',
              bgcolor: 'warning.lighter',
            }}
          >
            <Iconify icon="solar:user-rounded-bold-duotone" width={26} />
          </Box>

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
          href={paths.admin.wallets}
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
