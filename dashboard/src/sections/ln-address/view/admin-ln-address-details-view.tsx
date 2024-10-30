'use client';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';
import { displayLnAddress } from 'src/utils/lnurl';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetLnAddress } from 'src/actions/ln-addresses';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { LnAddressDetails } from '../ln-address-details';

// ----------------------------------------------------------------------

type Props = {
  id: string;
};

export function AdminLnAddressDetailsView({ id }: Props) {
  const { t } = useTranslate();
  const { lnAddress, lnAddressLoading, lnAddressError } = useGetLnAddress(id);

  const errors = [lnAddressError];
  const data = [lnAddress];
  const isLoading = [lnAddressLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission.READ_LN_ADDRESS]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={`${displayLnAddress(lnAddress!.username)}`}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('lightning_addresses'),
                  href: paths.admin.lnAddresses,
                },
                { name: lnAddress!.id.toUpperCase() },
              ]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <LnAddressDetails lnAddress={lnAddress!} isAdmin />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
