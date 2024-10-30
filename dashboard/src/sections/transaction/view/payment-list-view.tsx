'use client';

import type { TFunction } from 'i18next';
import type { Theme } from '@mui/material';
import type { LabelColor } from 'src/components/label';

import { mutate } from 'swr';

import { Box, Stack, Button, Tooltip, useTheme } from '@mui/material';

import { paths } from 'src/routes/paths';

import { useBoolean } from 'src/hooks/use-boolean';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useFetchFiatPrices } from 'src/actions/mempool-space';
import { useGetWalletBalance, useListWalletContacts, useListWalletPayments } from 'src/actions/user-wallet';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import { NewPaymentDialog, CleanTransactionsButton } from 'src/components/transactions';

import { TransactionType } from 'src/types/transaction';

import { TransactionList } from '../transaction-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'description', label: t('transaction_list.description') },
  { id: 'created_at', label: t('transaction_list.created') },
  { id: 'payment_time', label: t('transaction_list.settled') },
  { id: 'amount', label: t('transaction_list.amount') },
  { id: 'type', label: t('transaction_list.type') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

export const paymentTabs = (theme: Theme, t: TFunction) => [
  {
    title: t('transaction_list.tabs.total'),
    value: 'all',
    label: t('transaction_list.tabs.total'),
    color: 'default' as LabelColor,
    suffix: t('transaction_list.tabs.payments_suffix'),
    icon: 'solar:bill-list-bold-duotone',
    analyticColor: theme.palette.info.main,
  },
  {
    title: t('transaction_list.tabs.paid'),
    value: 'Settled',
    label: t('transaction_list.tabs.paid'),
    color: 'success' as LabelColor,
    suffix: t('transaction_list.tabs.payments_suffix'),
    icon: 'solar:bill-check-bold-duotone',
    analyticColor: theme.palette.success.main,
  },
  {
    title: t('transaction_list.tabs.pending'),
    value: 'Pending',
    label: t('transaction_list.tabs.pending'),
    color: 'warning' as LabelColor,
    suffix: t('transaction_list.tabs.payments_suffix'),
    icon: 'solar:sort-by-time-bold-duotone',
    analyticColor: theme.palette.warning.main,
  },
  {
    title: t('transaction_list.tabs.failed'),
    value: 'Failed',
    label: t('transaction_list.tabs.failed'),
    color: 'error' as LabelColor,
    suffix: t('transaction_list.tabs.payments_suffix'),
    icon: 'solar:bill-cross-bold-duotone',
    analyticColor: theme.palette.error.main,
  },
];

// ----------------------------------------------------------------------

export function PaymentListView() {
  const { t } = useTranslate();
  const theme = useTheme();

  const { payments, paymentsLoading, paymentsError } = useListWalletPayments();
  const { userBalance, userBalanceLoading, userBalanceError } = useGetWalletBalance();
  const { contacts, contactsLoading, contactsError } = useListWalletContacts();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();
  const newPayment = useBoolean();

  const errors = [paymentsError, userBalanceError, contactsError, fiatPricesError];
  const data = [payments, fiatPrices, userBalance, contacts];
  const isLoading = [paymentsLoading, userBalanceLoading, contactsLoading, fiatPricesLoading];

  const failed = shouldFail(errors, data, isLoading);

  const hasFailedPayments = payments?.some((payment) => payment.status === 'Failed') || false;

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('payments')}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('payments'),
              },
            ]}
            action={
              <Stack direction="row" spacing={1}>
                <Tooltip title={t('payment_list.clean_failed_payments')} placement="top" arrow>
                  <Box>
                    <CleanTransactionsButton
                      onSuccess={() => mutate(endpointKeys.userWallet.payments.list)}
                      buttonProps={{
                        color: 'error',
                        variant: 'contained',
                        startIcon: <Iconify icon="solar:trash-bin-trash-bold" />,
                        disabled: !hasFailedPayments,
                      }}
                      transactionType={TransactionType.PAYMENT}
                    >
                      {t('clean')}
                    </CleanTransactionsButton>
                  </Box>
                </Tooltip>
                <Button onClick={newPayment.onTrue} variant="contained" startIcon={<Iconify icon="mingcute:add-line" />}>
                  {t('new')}
                </Button>
              </Stack>
            }
            sx={{
              mb: { xs: 3, md: 5 },
            }}
          />

          <TransactionList
            data={payments!}
            tableHead={tableHead(t)}
            tabs={paymentTabs(theme, t)}
            transactionType={TransactionType.PAYMENT}
            href={paths.wallet.payment}
          />

          <NewPaymentDialog
            balance={userBalance!.available_msat}
            fiatPrices={fiatPrices!}
            open={newPayment.value}
            onClose={newPayment.onFalse}
            contacts={contacts!}
            onSuccess={() => mutate(endpointKeys.userWallet.payments.list)}
          />
        </>
      )}
    </DashboardContent>
  );
}
