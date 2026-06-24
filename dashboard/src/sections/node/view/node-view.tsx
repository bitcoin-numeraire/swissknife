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

import { shouldFail } from 'src/utils/errors';
import {
  getTotal,
  getCumulativeSeries,
  mergeAndSortTransactions as mergeTransactions,
} from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useSystemHealth } from 'src/actions/system';
import { useListPayments } from 'src/actions/payments';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListLnAddresses } from 'src/actions/ln-addresses';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RecentTransactions } from 'src/sections/transaction/recent-transactions';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

function healthColor(status?: string) {
  if (status === 'Operational') return 'success';
  if (status === 'Maintenance') return 'warning';
  return 'error';
}

function GapCard({ title, icon }: { title: string; icon: string }) {
  const { t } = useTranslate();

  return (
    <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
      <Stack spacing={2}>
        <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
          <Typography variant="h6">{title}</Typography>
          <Iconify icon={icon} width={24} sx={{ color: 'text.secondary' }} />
        </Stack>
        <Alert severity="info" variant="outlined">
          {t('node_view.backend_needed')}
        </Alert>
      </Stack>
    </Card>
  );
}

// ----------------------------------------------------------------------

export function NodeView() {
  const { t } = useTranslate();

  const { health, healthLoading, healthError } = useSystemHealth();
  const { payments, paymentsError } = useListPayments(100);
  const { invoices, invoicesError } = useListInvoices(100);
  const { lnAddresses, lnAddressesError } = useListLnAddresses(20);

  const errors = [healthError];
  const data = [health];
  const isLoading = [healthLoading];
  const failed = shouldFail(errors, data, isLoading);

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
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('node_view.heading')}
              links={[{ name: t('observe') }, { name: t('node_health') }]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Grid container spacing={3}>
              <Grid size={{ xs: 12, md: 5 }}>
                <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
                  <Stack spacing={3}>
                    <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                      <Stack>
                        <Typography variant="h6">{t('node_view.connection')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('node_view.read_only')}
                        </Typography>
                      </Stack>
                      <Label color={health?.is_healthy ? 'success' : 'error'}>
                        {health?.is_healthy ? t('node_view.operational') : t('node_view.unavailable')}
                      </Label>
                    </Stack>

                    {[
                      [t('node_view.database'), health?.database],
                      [t('node_view.ln_provider'), health?.ln_provider],
                    ].map(([label, value]) => (
                      <Stack key={label} spacing={1}>
                        <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                          <Typography variant="body2" color="text.secondary">
                            {label}
                          </Typography>
                          <Label color={healthColor(value)}>{value || t('node_view.unknown')}</Label>
                        </Stack>
                        <LinearProgress
                          variant="determinate"
                          value={value === 'Operational' ? 100 : value === 'Maintenance' ? 50 : 12}
                          color={healthColor(value)}
                        />
                      </Stack>
                    ))}

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
                  </Stack>
                </Card>
              </Grid>

              <Grid size={{ xs: 12, md: 7 }}>
                <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
                  <Stack spacing={3}>
                    <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                      <Typography variant="h6">{t('node_view.volume')}</Typography>
                      <Label color={volumeUnavailable ? 'warning' : 'info'}>
                        {volumeUnavailable ? t('node_view.partial') : t('node_view.client_computed')}
                      </Label>
                    </Stack>

                    {volumeUnavailable && (
                      <Alert severity="warning" variant="outlined">
                        {t('node_view.volume_unavailable')}
                      </Alert>
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
                <GapCard title={t('node_view.liquidity')} icon="solar:waterdrops-bold-duotone" />
              </Grid>
              <Grid size={{ xs: 12, md: 4 }}>
                <GapCard title={t('node_view.channels')} icon="solar:route-bold-duotone" />
              </Grid>
              <Grid size={{ xs: 12, md: 4 }}>
                <GapCard title={t('node_view.alerts')} icon="solar:bell-bing-bold-duotone" />
              </Grid>

              <Grid size={{ xs: 12 }}>
                <RecentTransactions isAdmin tableData={transactions.slice(0, 12)} />
              </Grid>
            </Grid>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
