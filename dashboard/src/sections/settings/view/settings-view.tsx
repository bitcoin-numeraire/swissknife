'use client';

import type { ReactElement } from 'react';

import { mutate } from 'swr';
import { useState } from 'react';

import Tab from '@mui/material/Tab';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';

import { useSearchParams } from 'src/routes/hooks';

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

type SettingsTab = 'lnaddress' | 'apikeys';

// ----------------------------------------------------------------------

export function SettingsView() {
  const { t } = useTranslate();
  const searchParams = useSearchParams();
  const requestedTab = searchParams.get('tab');
  const [activeTab, setActiveTab] = useState<SettingsTab>(
    requestedTab === 'apikeys' || requestedTab === 'lnaddress' ? requestedTab : 'lnaddress'
  );

  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();
  const { apiKeys, apiKeysLoading, apiKeysError } = useListWalletApiKeys();

  const errors = [lnAddressError, apiKeysError];
  const isLoading = [lnAddressLoading, apiKeysLoading];
  // `lnAddress` is legitimately null when no address is registered.
  const data = [apiKeys];

  const failed = shouldFail(errors, data, isLoading);
  const tabs: Array<{
    value: SettingsTab;
    label: string;
    icon: ReactElement;
  }> = [
    {
      value: 'lnaddress',
      label: t('settings_view.lightning_tab'),
      icon: <Iconify icon="solar:bolt-bold-duotone" width={24} />,
    },
    {
      value: 'apikeys',
      label: t('settings_view.api_keys_tab'),
      icon: <Iconify icon="solar:code-bold-duotone" width={24} />,
    },
  ];

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

          <Card sx={{ p: 3, mb: 3, borderRadius: 1 }}>
            <Stack
              direction={{ xs: 'column', md: 'row' }}
              spacing={3}
              sx={{ alignItems: { md: 'center' }, justifyContent: 'space-between' }}
            >
              <Stack spacing={0.75} sx={{ maxWidth: 640 }}>
                <Typography variant="h4">{t('settings_view.control_center')}</Typography>
                <Typography variant="body2" color="text.secondary">
                  {t('settings_view.subtitle')}
                </Typography>
              </Stack>

              <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5}>
                <SettingsStatus
                  icon="solar:bolt-bold-duotone"
                  title={t('settings_view.lightning_tab')}
                  value={
                    lnAddress
                      ? t('settings_view.address_ready')
                      : t('settings_view.address_missing')
                  }
                  color={lnAddress ? 'success.main' : 'warning.main'}
                />
                <SettingsStatus
                  icon="solar:key-minimalistic-square-bold-duotone"
                  title={t('settings_view.api_keys_tab')}
                  value={
                    apiKeys?.length
                      ? t('settings_view.tokens_count', { count: apiKeys.length })
                      : t('settings_view.no_tokens')
                  }
                  color={apiKeys?.length ? 'info.main' : 'text.disabled'}
                />
              </Stack>
            </Stack>
          </Card>

          <Tabs
            value={activeTab}
            onChange={(_, value: SettingsTab) => setActiveTab(value)}
            sx={{ mb: { xs: 3, md: 5 } }}
          >
            {tabs.map((tab) => (
              <Tab key={tab.value} label={tab.label} icon={tab.icon} value={tab.value} />
            ))}
          </Tabs>

          {activeTab === 'lnaddress' &&
            (lnAddress ? (
              <SettingsLnAddress lnAddress={lnAddress} />
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

          {activeTab === 'apikeys' && <SettingsApiKey apiKeys={apiKeys!} />}
        </>
      )}
    </DashboardContent>
  );
}

function SettingsStatus({
  icon,
  title,
  value,
  color,
}: {
  icon: string;
  title: string;
  value: string;
  color: string;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: 1.5,
          minWidth: { sm: 190 },
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
        }),
      ]}
    >
      <Stack direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
        <Iconify icon={icon} width={26} sx={{ color }} />
        <Stack sx={{ minWidth: 0 }}>
          <Typography variant="caption" color="text.secondary">
            {title}
          </Typography>
          <Typography variant="subtitle2" noWrap>
            {value}
          </Typography>
        </Stack>
      </Stack>
    </Box>
  );
}
