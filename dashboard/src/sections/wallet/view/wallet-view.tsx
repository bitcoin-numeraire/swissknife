'use client';

import type { Contact } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useMemo } from 'react';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid2';

import { shouldFail } from 'src/utils/errors';
import {
  getCumulativeSeries,
  getPercentageChange,
  mergeAndSortTransactions,
} from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetUserWallet } from 'src/actions/user-wallet';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { ErrorView } from 'src/components/error/error-view';
import { NewInvoiceCard, NewPaymentCard } from 'src/components/transactions';

import { RecentTransactions } from 'src/sections/transaction/recent-transactions';

import { Contacts } from '../contacts';
import { CurrentBalance } from '../current-balance';
import { BalanceOverview } from '../balance-overview';

// ----------------------------------------------------------------------

export function WalletView() {
  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();
  const { t } = useTranslate();

  const errors = [walletError, fiatPricesError];
  const data = [wallet, fiatPrices];
  const isLoading = [walletLoading, fiatPricesLoading];

  const incomeSeries = useMemo(
    () => getCumulativeSeries(wallet?.invoices || []),
    [wallet?.invoices]
  );
  const expensesSeries = useMemo(
    () => getCumulativeSeries(wallet?.payments || []),
    [wallet?.payments]
  );

  const percentageChangeIncome = useMemo(
    () => getPercentageChange(wallet?.invoices || []),
    [wallet?.invoices]
  );
  const percentageChangeExpenses = useMemo(
    () => getPercentageChange(wallet?.payments || []),
    [wallet?.payments]
  );

  const allTransactions = useMemo(
    () => mergeAndSortTransactions(wallet?.invoices || [], wallet?.payments || []),
    [wallet?.invoices, wallet?.payments]
  );

  const contacts: Contact[] = useMemo(() => wallet?.contacts || [], [wallet?.contacts]);

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent maxWidth="xl">
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <Grid container spacing={3}>
          <Grid size={{ xs: 12, md: 7, lg: 8 }}>
            <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
              <BalanceOverview
                contacts={contacts}
                fiatPrices={fiatPrices!}
                title={t('wallet_view.total_balance')}
                tooltipTitle={t('wallet_view.balance_tooltip')}
                totalBalance={wallet?.balance.available_msat}
                income={{
                  value: 'income',
                  label: t('wallet_view.income'),
                  percent: percentageChangeIncome,
                  total: wallet!.balance.received_msat,
                  color: 'success',
                  tooltipTitle: t('wallet_view.income_tooltip'),
                  series: incomeSeries,
                }}
                expenses={{
                  value: 'expenses',
                  label: t('wallet_view.expenses'),
                  percent: percentageChangeExpenses,
                  total: wallet!.balance.sent_msat,
                  color: 'error',
                  tooltipTitle: t('wallet_view.expenses_tooltip'),
                  series: expensesSeries,
                }}
              />

              <RecentTransactions tableData={allTransactions.slice(0, 20)} />
            </Box>
          </Grid>

          <Grid size={{ xs: 12, md: 5, lg: 4 }}>
            <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
              <CurrentBalance wallet={wallet!} fiatPrices={fiatPrices!} />
              <NewPaymentCard
                title={t('wallet_view.quick_transfer')}
                fiatPrices={fiatPrices!}
                contacts={contacts}
                balance={wallet!.balance.available_msat}
                onSuccess={() => mutate(endpointKeys.userWallet.get)}
              />
              <NewInvoiceCard
                title={t('wallet_view.receive_bitcoin')}
                lnAddress={wallet?.ln_address}
                fiatPrices={fiatPrices!}
                onSuccess={() => mutate(endpointKeys.userWallet.get)}
              />
              <Contacts
                title={t('wallet_view.contacts')}
                list={contacts}
                fiatPrices={fiatPrices!}
              />
            </Box>
          </Grid>
        </Grid>
      )}
    </DashboardContent>
  );
}
