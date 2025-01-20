'use client';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetWalletInvoice } from 'src/actions/user-wallet';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { InvoiceDetails } from '../invoice-details';

// ----------------------------------------------------------------------

type Props = {
  id: string;
};

export function InvoiceDetailsView({ id }: Props) {
  const { t } = useTranslate();

  const { invoice, invoiceLoading, invoiceError } = useGetWalletInvoice(id);

  const errors = [invoiceError];
  const data = [invoice];
  const isLoading = [invoiceLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={`${t('invoice')}: ${truncateText(invoice!.id.toUpperCase(), 8)}`}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('invoices'),
                href: paths.wallet.invoices,
              },
              { name: invoice!.id.toUpperCase() },
            ]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <InvoiceDetails invoice={invoice!} />
        </>
      )}
    </DashboardContent>
  );
}
