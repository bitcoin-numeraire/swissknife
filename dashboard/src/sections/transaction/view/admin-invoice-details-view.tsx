'use client';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useGetInvoice } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { InvoiceDetails } from '../invoice-details';

// ----------------------------------------------------------------------

type Props = {
  id: string;
};

export function AdminInvoiceDetailsView({ id }: Props) {
  const { t } = useTranslate();
  const { invoice, invoiceLoading, invoiceError } = useGetInvoice(id);

  const errors = [invoiceError];
  const data = [invoice];
  const isLoading = [invoiceLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission.READ_TRANSACTION]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={`${t('invoice')}: ${truncateText(invoice!.id.toUpperCase(), 8)}`}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('invoices'),
                  href: paths.admin.invoices,
                },
                { name: invoice!.id.toUpperCase() },
              ]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <InvoiceDetails invoice={invoice!} isAdmin />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
