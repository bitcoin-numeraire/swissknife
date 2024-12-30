'use client';

import { useMemo } from 'react';

import { Box, Grid2 } from '@mui/material';

import { shouldFail } from 'src/utils/errors';
import {
  getTotal,
  getCumulativeSeries,
  getPercentageChange,
  mergeAndSortTransactions as mergeTransactions,
} from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useListPayments } from 'src/actions/payments';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListLnAddresses } from 'src/actions/ln-addresses';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { BalanceOverview } from 'src/sections/wallet/balance-overview';
import { RecentTransactions } from 'src/sections/transaction/recent-transactions';

import { RoleBasedGuard } from 'src/auth/guard';

import { LnAddresses } from '../ln-addresses';

// ----------------------------------------------------------------------

export function NodeView() {
  const { t } = useTranslate();

  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();
  const { payments, paymentsLoading, paymentsError } = useListPayments();
  const { invoices, invoicesLoading, invoicesError } = useListInvoices();
  const { lnAddresses, lnAddressesLoading, lnAddressesError } = useListLnAddresses();

  const errors = [fiatPricesError, invoicesError, paymentsError, lnAddressesError];
  const data = [fiatPrices, invoices, payments, lnAddresses];
  const isLoading = [fiatPricesLoading, invoicesLoading, paymentsLoading, lnAddressesLoading];

  const incomeSeries = useMemo(() => getCumulativeSeries(invoices || []), [invoices]);
  const expensesSeries = useMemo(() => getCumulativeSeries(payments || []), [payments]);
  const percentageChangeIncome = useMemo(() => getPercentageChange(invoices || []), [invoices]);
  const percentageChangeExpenses = useMemo(() => getPercentageChange(payments || []), [payments]);
  const totalInvoices = useMemo(() => getTotal(invoices || []), [invoices]);
  const totalPayments = useMemo(() => getTotal(payments || []), [payments]);

  const transactions = useMemo(
    () => mergeTransactions(invoices || [], payments || []),
    [invoices, payments]
  );

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard
        permissions={[
          Permission['READ:TRANSACTION'],
          Permission['READ:LN_NODE'],
          Permission['READ:LN_ADDRESS'],
        ]}
        hasContent
      >
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('node_view.heading')}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('node_management'),
                },
              ]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Grid2 container spacing={3}>
              <Grid2 size={{ xs: 12, md: 7, lg: 8 }}>
                <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
                  <BalanceOverview
                    isAdmin
                    fiatPrices={fiatPrices!}
                    title={t('node_view.volume')}
                    tooltipTitle={t('node_view.volume_tooltip')}
                    income={{
                      value: 'income',
                      label: t('node_view.received'),
                      tooltipTitle: t('node_view.received_tooltip'),
                      percent: percentageChangeIncome,
                      total: totalInvoices,
                      color: 'success',
                      series: incomeSeries,
                    }}
                    expenses={{
                      value: 'expenses',
                      label: t('node_view.sent'),
                      tooltipTitle: t('node_view.sent_tooltip'),
                      percent: percentageChangeExpenses,
                      total: totalPayments,
                      color: 'error',
                      series: expensesSeries,
                    }}
                  />

                  <RecentTransactions isAdmin tableData={transactions.slice(0, 20)} />
                </Box>
              </Grid2>

              <Grid2 size={{ xs: 12, md: 5, lg: 4 }}>
                <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
                  <LnAddresses
                    subheader={t('node_view.registered_ln_addresses', {
                      count: lnAddresses!.length,
                    })}
                    list={lnAddresses!.slice(-20)}
                  />
                </Box>
              </Grid2>
            </Grid2>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
