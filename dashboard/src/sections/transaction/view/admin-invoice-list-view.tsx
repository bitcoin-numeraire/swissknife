'use client';

import type { TFunction } from 'i18next';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import { useTheme } from '@mui/material/styles';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { Permission } from 'src/lib/swissknife';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { NewInvoiceDialog } from 'src/components/transactions';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { TransactionType } from 'src/types/transaction';

import { invoiceTabs } from './invoice-list-view';
import { TransactionList } from '../transaction-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'description', label: t('transaction_list.description') },
  { id: 'wallet_id', label: t('transaction_list.wallet') },
  { id: 'created_at', label: t('transaction_list.created') },
  { id: 'expires_at', label: t('transaction_list.expires') },
  { id: 'payment_time', label: t('transaction_list.settled') },
  { id: 'amount', label: t('transaction_list.amount') },
  { id: 'type', label: t('transaction_list.type') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

export function AdminInvoiceListView() {
  const { t } = useTranslate();
  const theme = useTheme();
  const newInvoice = useBoolean();

  const { invoices, invoicesLoading, invoicesError } = useListInvoices();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();

  const errors = [invoicesError, fiatPricesError];
  const data = [invoices, fiatPrices];
  const isLoading = [invoicesLoading, fiatPricesLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission['READ:TRANSACTION']]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('invoices')}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('invoices'),
                },
              ]}
              action={
                <Stack direction="row" spacing={1}>
                  <Button
                    onClick={newInvoice.onTrue}
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
              data={invoices!}
              isAdmin
              tableHead={tableHead(t)}
              tabs={invoiceTabs(theme, t)}
              transactionType={TransactionType.INVOICE}
              href={paths.admin.invoice}
            />

            <NewInvoiceDialog
              fiatPrices={fiatPrices!}
              open={newInvoice.value}
              onClose={newInvoice.onFalse}
              onSuccess={() => mutate(endpointKeys.invoices.list)}
              isAdmin
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
