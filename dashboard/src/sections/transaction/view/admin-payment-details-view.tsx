'use client';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useGetPayment } from 'src/actions/payments';
import { DashboardContent } from 'src/layouts/dashboard';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { PaymentDetails } from '../payment-details';

// ----------------------------------------------------------------------

type Props = {
  id: string;
};

export function AdminPaymentDetailsView({ id }: Props) {
  const { t } = useTranslate();
  const { payment, paymentLoading, paymentError } = useGetPayment(id);

  const errors = [paymentError];
  const data = [payment];
  const isLoading = [paymentLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission['READ:TRANSACTION']]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={`${t('payment')}: ${truncateText(payment!.id.toUpperCase(), 8)}`}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('payments'),
                  href: paths.admin.payments,
                },
                { name: payment!.id.toUpperCase() },
              ]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <PaymentDetails payment={payment!} isAdmin />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
