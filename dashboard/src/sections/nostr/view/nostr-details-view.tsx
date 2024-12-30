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

import { NostrDetails } from '../nostr-details';

// ----------------------------------------------------------------------

export function NostrDetailsView() {
  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();
  const { t } = useTranslate();

  const errors = [lnAddressError];
  const isLoading = [lnAddressLoading];

  const failed = shouldFail(errors, [lnAddress], isLoading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('nostr_address')}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('nostr_address'),
              },
            ]}
            sx={{
              mb: { xs: 3, md: 5 },
            }}
          />

          {lnAddress?.ln_address ? (
            <NostrDetails lnAddress={lnAddress.ln_address} />
          ) : (
            <Welcome
              description={t('register_ln_address.missing_lightning_address_nostr')}
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
