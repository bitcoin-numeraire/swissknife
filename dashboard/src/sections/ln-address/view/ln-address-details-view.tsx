'use client';

import { mutate } from 'swr';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetWalletLnAddress } from 'src/actions/user-wallet';

import { Welcome } from 'src/components/app/welcome';
import { ErrorView } from 'src/components/error/error-view';
import { RegisterLnAddressForm } from 'src/components/ln-address';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs/custom-breadcrumbs';

import { LnAddressDetails } from '../ln-address-details';

// ----------------------------------------------------------------------

export function LnAddressDetailsView() {
  const { t } = useTranslate();
  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();

  const errors = [lnAddressError];
  const loading = [lnAddressLoading];

  const failed = shouldFail(errors, [lnAddress], loading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={loading} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('lightning_address')}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('lightning_address'),
              },
            ]}
            sx={{
              mb: { xs: 3, md: 5 },
            }}
          />

          {lnAddress!.ln_address ? (
            <LnAddressDetails lnAddress={lnAddress!.ln_address} />
          ) : (
            <Welcome
              description={t('register_ln_address.register_lightning_address_welcome')}
              img={
                <img src="/assets/icons/bitcoin/ic-bitcoin-lightning.svg" alt="Lightning logo" />
              }
              action={
                <RegisterLnAddressForm
                  onSuccess={() => mutate(endpointKeys.userWallet.lnAddress.get)}
                />
              }
            />
          )}
        </>
      )}
    </DashboardContent>
  );
}
