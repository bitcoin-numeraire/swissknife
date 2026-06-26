'use client';

import { useMemo } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';

import {
  getTotal,
  getCumulativeSeries,
  mergeAndSortTransactions as mergeTransactions,
} from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { useSystemHealth } from 'src/actions/system';
import { useListPayments } from 'src/actions/payments';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { Permission, HealthStatus } from 'src/lib/swissknife';
import { useListLnAddresses } from 'src/actions/ln-addresses';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RecentTransactions } from 'src/sections/transaction/recent-transactions';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

function healthColor(status?: string) {
  if (status === 'Operational') return 'success';
  if (status === 'Maintenance') return 'warning';
  return 'error';
}

function healthProgress(status?: string) {
  if (status === 'Operational') return 100;
  if (status === 'Maintenance') return 50;
  return 12;
}

function HealthMetric({ label, value }: { label: string; value?: string }) {
  const { t } = useTranslate();

  return (
    <Stack spacing={1}>
      <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
        <Typography variant="body2" color="text.secondary">
          {label}
        </Typography>
        <Label color={healthColor(value)}>{value || t('node_view.unknown')}</Label>
      </Stack>
      <LinearProgress
        variant="determinate"
        value={healthProgress(value)}
        color={healthColor(value)}
      />
    </Stack>
  );
}

function GapCard({ title, icon, body }: { title: string; icon: string; body?: string }) {
  const { t } = useTranslate();

  return (
    <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
      <Stack spacing={2}>
        <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
          <Typography variant="h6">{title}</Typography>
          <Iconify icon={icon} width={24} sx={{ color: 'text.secondary' }} />
        </Stack>
        <Alert severity="info" variant="outlined">
          {body || t('node_view.backend_needed')}
        </Alert>
      </Stack>
    </Card>
  );
}

// ----------------------------------------------------------------------

export function NodeView() {
  const { t } = useTranslate();

  const { health, healthLoading, healthError, healthDegraded, healthDegradedReason } =
    useSystemHealth();
  const { payments, paymentsError } = useListPayments(100);
  const { invoices, invoicesError } = useListInvoices(100);
  const { lnAddresses, lnAddressesError } = useListLnAddresses(20);

  const effectiveHealth = health ?? {
    is_healthy: false,
    database: HealthStatus.MAINTENANCE,
    ln_provider: HealthStatus.UNAVAILABLE,
  };
  const healthUnavailable = !!healthError || healthDegraded || !health;

  const incomeSeries = useMemo(() => getCumulativeSeries(invoices || []), [invoices]);
  const expensesSeries = useMemo(() => getCumulativeSeries(payments || []), [payments]);
  const totalInvoices = useMemo(() => getTotal(invoices || []), [invoices]);
  const totalPayments = useMemo(() => getTotal(payments || []), [payments]);
  const transactions = useMemo(
    () => mergeTransactions(invoices || [], payments || []),
    [invoices, payments]
  );
  const volumeUnavailable = paymentsError || invoicesError;

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_LN_NODE]} hasContent>
        {healthLoading && !health ? (
          <LinearProgress />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('node_view.heading')}
              links={[{ name: t('observe') }, { name: t('node_health') }]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            {healthUnavailable && (
              <Alert severity="warning" variant="outlined" sx={{ mb: 3 }}>
                {healthDegradedReason === 'timeout'
                  ? t('node_view.health_timeout')
                  : t('node_view.health_unavailable')}
              </Alert>
            )}

            <Grid container spacing={3}>
              <Grid size={{ xs: 12, md: 4 }}>
                <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
                  <Stack spacing={3}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'flex-start', justifyContent: 'space-between' }}
                    >
                      <Stack>
                        <Typography variant="h6">{t('node_view.backend')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('node_view.backend_body')}
                        </Typography>
                      </Stack>
                      <Label color={healthUnavailable ? 'warning' : 'success'}>
                        {healthUnavailable ? t('node_view.partial') : t('node_view.operational')}
                      </Label>
                    </Stack>

                    <HealthMetric label={t('node_view.database')} value={effectiveHealth.database} />

                    <Divider />

                    <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                      <Typography variant="body2" color="text.secondary">
                        {t('node_view.health_endpoint')}
                      </Typography>
                      <Label color={health ? 'success' : 'warning'}>
                        {health ? t('node_view.available') : t('node_view.unavailable')}
                      </Label>
                    </Stack>
                  </Stack>
                </Card>
              </Grid>

              <Grid size={{ xs: 12, md: 4 }}>
                <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
                  <Stack spacing={3}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'flex-start', justifyContent: 'space-between' }}
                    >
                      <Stack>
                        <Typography variant="h6">{t('node_view.provider')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('node_view.provider_body')}
                        </Typography>
                      </Stack>
                      <Label color={healthColor(effectiveHealth.ln_provider)}>
                        {effectiveHealth.ln_provider || t('node_view.unknown')}
                      </Label>
                    </Stack>

                    <HealthMetric
                      label={t('node_view.ln_provider')}
                      value={effectiveHealth.ln_provider}
                    />
                  </Stack>
                </Card>
              </Grid>

              <Grid size={{ xs: 12, md: 4 }}>
                <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
                  <Stack spacing={3}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'flex-start', justifyContent: 'space-between' }}
                    >
                      <Stack>
                        <Typography variant="h6">{t('node_view.activity')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('node_view.activity_body')}
                        </Typography>
                      </Stack>
                      <Label color={volumeUnavailable ? 'warning' : 'info'}>
                        {volumeUnavailable ? t('node_view.partial') : t('node_view.client_computed')}
                      </Label>
                    </Stack>

                    {volumeUnavailable && (
                      <Alert severity="warning" variant="outlined">
                        {t('node_view.volume_unavailable')}
                      </Alert>
                    )}

                    {lnAddressesError ? (
                      <Alert severity="warning" variant="outlined">
                        {t('node_view.ln_addresses_unavailable')}
                      </Alert>
                    ) : (
                      <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                        <Typography variant="body2" color="text.secondary">
                          {t('node_view.registered_addresses')}
                        </Typography>
                        <Typography variant="subtitle2">{lnAddresses?.length ?? 0}</Typography>
                      </Stack>
                    )}

                    <Grid container spacing={2}>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <Box sx={{ p: 2, borderRadius: 1, bgcolor: 'background.neutral' }}>
                          <Typography variant="body2" color="text.secondary">
                            {t('node_view.received')}
                          </Typography>
                          <Typography variant="h4">{Math.round(totalInvoices / 1000)} sats</Typography>
                        </Box>
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <Box sx={{ p: 2, borderRadius: 1, bgcolor: 'background.neutral' }}>
                          <Typography variant="body2" color="text.secondary">
                            {t('node_view.sent')}
                          </Typography>
                          <Typography variant="h4">{Math.round(totalPayments / 1000)} sats</Typography>
                        </Box>
                      </Grid>
                    </Grid>

                    <Divider />

                    <Stack direction="row" spacing={1}>
                      <Label color="success">{incomeSeries[0].data.length} in</Label>
                      <Label color="warning">{expensesSeries[0].data.length} out</Label>
                    </Stack>
                  </Stack>
                </Card>
              </Grid>

              <Grid size={{ xs: 12, md: 4 }}>
                <GapCard
                  title={t('node_view.liquidity')}
                  icon="solar:waterdrops-bold-duotone"
                  body={t('node_view.provider_gap')}
                />
              </Grid>
              <Grid size={{ xs: 12, md: 4 }}>
                <GapCard
                  title={t('node_view.channels')}
                  icon="solar:route-bold-duotone"
                  body={t('node_view.provider_gap')}
                />
              </Grid>
              <Grid size={{ xs: 12, md: 4 }}>
                <GapCard
                  title={t('node_view.alerts')}
                  icon="solar:bell-bing-bold-duotone"
                  body={t('node_view.backend_gap')}
                />
              </Grid>

              <Grid size={{ xs: 12 }}>
                <Stack spacing={2}>
                  <Stack>
                    <Typography variant="h6">{t('node_view.activity_log')}</Typography>
                    <Typography variant="body2" color="text.secondary">
                      {t('node_view.activity_log_body')}
                    </Typography>
                  </Stack>
                  <RecentTransactions isAdmin tableData={transactions.slice(0, 12)} />
                </Stack>
              </Grid>
            </Grid>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
