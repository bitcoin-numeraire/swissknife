'use client';

import type { TFunction } from 'i18next';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import { Stack, Button, useTheme } from '@mui/material';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { Permission } from 'src/lib/swissknife';
import { useListPayments } from 'src/actions/payments';
import { DashboardContent } from 'src/layouts/dashboard';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { NewPaymentDialog } from 'src/components/transactions';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { TransactionType } from 'src/types/transaction';

import { paymentTabs } from './payment-list-view';
import { TransactionList } from '../transaction-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'description', label: t('transaction_list.description') },
  { id: 'wallet_id', label: t('transaction_list.wallet') },
  { id: 'created_at', label: t('transaction_list.created') },
  { id: 'payment_time', label: t('transaction_list.settled') },
  { id: 'amount', label: t('transaction_list.amount') },
  { id: 'type', label: t('transaction_list.type') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

export function AdminPaymentListView() {
  const { t } = useTranslate();
  const theme = useTheme();
  const { payments, paymentsLoading, paymentsError } = useListPayments();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();
  const newPayment = useBoolean();

  const errors = [paymentsError, fiatPricesError];
  const data = [payments, fiatPrices];
  const isLoading = [paymentsLoading, fiatPricesLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission['READ:TRANSACTION']]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('payments')}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('payments'),
                },
              ]}
              action={
                <Stack direction="row" spacing={1}>
                  <Button
                    onClick={newPayment.onTrue}
                    variant="contained"
                    startIcon={<Iconify icon="mingcute:add-line" />}
                  >
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
              href={paths.admin.payment}
              isAdmin
            />

            <NewPaymentDialog
              fiatPrices={fiatPrices!}
              open={newPayment.value}
              onClose={newPayment.onFalse}
              onSuccess={() => mutate(endpointKeys.payments.list)}
              isAdmin
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
