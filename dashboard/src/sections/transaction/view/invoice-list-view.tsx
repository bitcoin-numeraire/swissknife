'use client';

import type { TFunction } from 'i18next';
import type { Theme } from '@mui/material/styles';
import type { LabelColor } from 'src/components/label';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import { Box, Tooltip } from '@mui/material';
import { useTheme } from '@mui/material/styles';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useFetchFiatPrices } from 'src/actions/mempool-space';
import { useListWalletInvoices } from 'src/actions/user-wallet';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import { NewInvoiceDialog, CleanTransactionsButton } from 'src/components/transactions';

import { TransactionType } from 'src/types/transaction';

import { TransactionList } from '../transaction-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'description', label: t('transaction_list.description') },
  { id: 'created_at', label: t('transaction_list.created') },
  { id: 'expires_at', label: t('transaction_list.expires') },
  { id: 'payment_time', label: t('transaction_list.settled') },
  { id: 'amount', label: t('transaction_list.amount') },
  { id: 'type', label: t('transaction_list.type') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

export const invoiceTabs = (theme: Theme, t: TFunction) => [
  {
    title: t('transaction_list.tabs.total'),
    value: 'all',
    label: t('transaction_list.tabs.total'),
    color: 'default' as LabelColor,
    suffix: t('transaction_list.tabs.invoices_suffix'),
    icon: 'solar:bill-list-bold-duotone',
    analyticColor: theme.palette.info.main,
  },
  {
    title: t('transaction_list.tabs.paid'),
    value: 'Settled',
    label: t('transaction_list.tabs.paid'),
    color: 'success' as LabelColor,
    suffix: t('transaction_list.tabs.invoices_suffix'),
    icon: 'solar:bill-check-bold-duotone',
    analyticColor: theme.palette.success.main,
  },
  {
    title: t('transaction_list.tabs.pending'),
    value: 'Pending',
    label: t('transaction_list.tabs.pending'),
    color: 'warning' as LabelColor,
    suffix: t('transaction_list.tabs.invoices_suffix'),
    icon: 'solar:sort-by-time-bold-duotone',
    analyticColor: theme.palette.warning.main,
  },
  {
    title: t('transaction_list.tabs.expired'),
    value: 'Expired',
    label: t('transaction_list.tabs.expired'),
    color: 'error' as LabelColor,
    suffix: t('transaction_list.tabs.invoices_suffix'),
    icon: 'solar:bell-bing-bold-duotone',
    analyticColor: theme.palette.error.main,
  },
];

// ----------------------------------------------------------------------

export function InvoiceListView() {
  const { t } = useTranslate();
  const theme = useTheme();
  const newInvoice = useBoolean();

  const { invoices, invoicesLoading, invoicesError, invoicesMutate } = useListWalletInvoices();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();

  const errors = [invoicesError, fiatPricesError];
  const data = [invoices, fiatPrices];
  const isLoading = [invoicesLoading, fiatPricesLoading];

  const failed = shouldFail(errors, data, isLoading);

  const hasExpiredInvoices = invoices?.some((invoice) => invoice.status === 'Expired') || false;

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('invoices')}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('invoices'),
              },
            ]}
            action={
              <Stack direction="row" spacing={1}>
                <Tooltip title={t('invoice_list.clean_expired_invoices')} placement="top" arrow>
                  <Box>
                    <CleanTransactionsButton
                      onSuccess={invoicesMutate}
                      buttonProps={{
                        color: 'error',
                        variant: 'contained',
                        startIcon: <Iconify icon="solar:trash-bin-trash-bold" />,
                        disabled: !hasExpiredInvoices,
                      }}
                      transactionType={TransactionType.INVOICE}
                    >
                      {t('clean')}
                    </CleanTransactionsButton>
                  </Box>
                </Tooltip>
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
            tableHead={tableHead(t)}
            tabs={invoiceTabs(theme, t)}
            transactionType={TransactionType.INVOICE}
            href={paths.wallet.invoice}
          />

          <NewInvoiceDialog
            fiatPrices={fiatPrices!}
            open={newInvoice.value}
            onClose={newInvoice.onFalse}
            onSuccess={() => mutate(endpointKeys.userWallet.invoices.list)}
          />
        </>
      )}
    </DashboardContent>
  );
}
