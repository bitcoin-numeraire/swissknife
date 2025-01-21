'use client';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetWalletPayment } from 'src/actions/user-wallet';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { PaymentDetails } from '../payment-details';

// ----------------------------------------------------------------------

type Props = {
  id: string;
};

export function PaymentDetailsView({ id }: Props) {
  const { t } = useTranslate();

  const { payment, paymentLoading, paymentError } = useGetWalletPayment(id);

  const errors = [paymentError];
  const data = [payment];
  const isLoading = [paymentLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={`${t('payment')}: ${truncateText(payment!.id.toUpperCase(), 8)}`}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('payments'),
                href: paths.wallet.payments,
              },
              { name: payment!.id.toUpperCase() },
            ]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <PaymentDetails payment={payment!} />
        </>
      )}
    </DashboardContent>
  );
}
