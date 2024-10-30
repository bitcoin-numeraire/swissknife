'use client';

import { useMemo } from 'react';

import { Box } from '@mui/material';
import Grid from '@mui/material/Unstable_Grid2';

import { shouldFail } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';
import { getTotal, getCumulativeSeries, getPercentageChange, mergeAndSortTransactions } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useListPayments } from 'src/actions/payments';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListLnAddresses } from 'src/actions/ln-addresses';
import { useFetchFiatPrices } from 'src/actions/mempool-space';
import { useGetLSPs, useGetNodeInfo, useGetCurrentLSP } from 'src/actions/ln-node';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { BalanceOverview } from 'src/sections/wallet/balance-overview';
import { RecentTransactions } from 'src/sections/transaction/recent-transactions';

import { RoleBasedGuard } from 'src/auth/guard';

import { LSPList } from '../lsp-list';
import { LnAddresses } from '../ln-addresses';
import { SignMessage } from '../sign-message';
import { VerifyMessage } from '../verify-message';
import { CurrentBalance } from '../current-balance';

// ----------------------------------------------------------------------

export function BreezNodeView() {
  const { t } = useTranslate();

  const { nodeInfo, nodeInfoLoading, nodeInfoError } = useGetNodeInfo();
  const { lsps, lspsLoading, lspsError } = useGetLSPs();
  const { currentLSP, currentLSPLoading, currentLSPError } = useGetCurrentLSP();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();
  const { payments, paymentsLoading, paymentsError } = useListPayments();
  const { invoices, invoicesLoading, invoicesError } = useListInvoices();
  const { lnAddresses, lnAddressesLoading, lnAddressesError } = useListLnAddresses();

  const errors = [nodeInfoError, fiatPricesError, invoicesError, paymentsError, lnAddressesError, lspsError, currentLSPError];
  const data = [nodeInfo, fiatPrices, invoices, payments, lnAddresses, lsps, currentLSP];
  const isLoading = [
    nodeInfoLoading,
    fiatPricesLoading,
    invoicesLoading,
    paymentsLoading,
    lnAddressesLoading,
    lspsLoading,
    currentLSPLoading,
  ];

  const incomeSeries = useMemo(() => getCumulativeSeries(invoices || []), [invoices]);
  const expensesSeries = useMemo(() => getCumulativeSeries(payments || []), [payments]);
  const percentageChangeIncome = useMemo(() => getPercentageChange(invoices || []), [invoices]);
  const percentageChangeExpenses = useMemo(() => getPercentageChange(payments || []), [payments]);
  const totalInvoices = useMemo(() => getTotal(invoices || []), [invoices]);
  const totalPayments = useMemo(() => getTotal(payments || []), [payments]);

  const transactions = useMemo(() => mergeAndSortTransactions(invoices || [], payments || []), [invoices, payments]);

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_TRANSACTION, Permission.READ_LN_NODE, Permission.READ_LN_ADDRESS]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={`${t('breez_node_view.node')}: ${truncateText(nodeInfo!.id, 20)}`}
              icon={<Box component="img" src="/assets/icons/brands/ic-brand-greenlight.svg" sx={{ height: { xs: 24, md: 32 } }} />}
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

            <Grid container spacing={3}>
              <Grid xs={12} md={7} lg={8}>
                <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
                  <BalanceOverview
                    isAdmin
                    fiatPrices={fiatPrices!}
                    title={t('breez_node_view.total_balance')}
                    tooltipTitle={t('breez_node_view.total_balance_tooltip')}
                    totalBalance={nodeInfo!.channels_balance_msat + nodeInfo!.onchain_balance_msat}
                    income={{
                      value: 'income',
                      label: t('breez_node_view.total_received'),
                      tooltipTitle: t('breez_node_view.total_received_tooltip'),
                      percent: percentageChangeIncome,
                      total: totalInvoices,
                      color: 'success',
                      series: incomeSeries,
                    }}
                    expenses={{
                      value: 'expenses',
                      label: t('breez_node_view.total_sent'),
                      tooltipTitle: t('breez_node_view.total_sent_tooltip'),
                      percent: percentageChangeExpenses,
                      total: totalPayments,
                      color: 'error',
                      series: expensesSeries,
                    }}
                  />

                  <RecentTransactions isAdmin tableData={transactions.slice(0, 20)} />
                </Box>
              </Grid>

              <Grid xs={12} md={5} lg={4}>
                <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
                  <CurrentBalance title={t('breez_node_view.current_balance')} nodeInfo={nodeInfo!} />
                  <LnAddresses
                    subheader={t('breez_node_view.registered_ln_addresses', { count: lnAddresses!.length })}
                    list={lnAddresses!.slice(-20)}
                  />
                </Box>
              </Grid>

              <Grid xs={12}>
                <LSPList
                  currentLSP={currentLSP!.pubkey}
                  title={t('breez_node_view.available_lsps')}
                  tableData={lsps!}
                  tableLabels={[
                    { id: 'details', label: t('breez_node_view.lsp_table_labels.details') },
                    { id: 'ID', label: t('breez_node_view.lsp_table_labels.id') },
                    { id: 'host', label: t('breez_node_view.lsp_table_labels.host') },
                    { id: 'baseFee', label: t('breez_node_view.lsp_table_labels.base_fee') },
                    { id: 'feeRate', label: t('breez_node_view.lsp_table_labels.fee_rate') },
                    { id: 'timelockDelta', label: t('breez_node_view.lsp_table_labels.timelock_delta') },
                    { id: 'minHTLC', label: t('breez_node_view.lsp_table_labels.min_htlc') },
                    { id: 'status', label: t('breez_node_view.lsp_table_labels.status') },
                    { id: '' },
                  ]}
                />
              </Grid>

              <Grid xs={12} md={6}>
                <SignMessage />
              </Grid>

              <Grid xs={12} md={6}>
                <VerifyMessage />
              </Grid>
            </Grid>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
