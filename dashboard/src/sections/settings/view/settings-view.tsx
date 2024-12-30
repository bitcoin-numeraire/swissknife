'use client';

import { mutate } from 'swr';
import { useTabs } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletApiKeys, useGetWalletLnAddress } from 'src/actions/user-wallet';

import { Welcome } from 'src/components/app';
import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error';
import { RegisterLnAddressForm } from 'src/components/ln-address';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { SettingsApiKey } from '../settings-api-key';
import { SettingsLnAddress } from '../settings-ln-address';

// ----------------------------------------------------------------------

const TABS = [
  {
    value: 'lnaddress',
    label: 'Lightning Address',
    icon: <Iconify icon="solar:bolt-bold-duotone" width={24} />,
  },
  {
    value: 'apikeys',
    label: 'API Keys',
    icon: <Iconify icon="solar:code-bold-duotone" width={24} />,
  },
];

// ----------------------------------------------------------------------

export function SettingsView() {
  const tabs = useTabs('lnaddress');
  const { t } = useTranslate();

  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();
  const { apiKeys, apiKeysLoading, apiKeysError } = useListWalletApiKeys();

  const errors = [lnAddressError, apiKeysError];
  const isLoading = [lnAddressLoading, apiKeysLoading];
  const data = [apiKeys, lnAddress];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('settings')}
            links={[
              {
                name: t('settings_view.manage_account'),
              },
            ]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <Tabs value={tabs.value} onChange={tabs.onChange} sx={{ mb: { xs: 3, md: 5 } }}>
            {TABS.map((tab) => (
              <Tab key={tab.value} label={tab.label} icon={tab.icon} value={tab.value} />
            ))}
          </Tabs>

          {tabs.value === 'lnaddress' &&
            (lnAddress?.ln_address ? (
              <SettingsLnAddress lnAddress={lnAddress.ln_address} />
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
            ))}

          {tabs.value === 'apikeys' && <SettingsApiKey apiKeys={apiKeys!} />}
        </>
      )}
    </DashboardContent>
  );
}
