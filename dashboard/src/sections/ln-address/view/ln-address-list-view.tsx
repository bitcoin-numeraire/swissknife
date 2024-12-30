'use client';

import type { TFunction } from 'i18next';
import type { LabelColor } from 'src/components/label';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import { useTheme } from '@mui/material/styles';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListLnAddresses } from 'src/actions/ln-addresses';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { RegisterLnAddressDialog } from 'src/components/ln-address';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { LnAddressList } from '../ln-address-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'username', label: t('ln_address_list.username') },
  { id: 'wallet_id', label: t('ln_address_list.wallet') },
  { id: 'nostr_pubkey', label: t('ln_address_list.nostr_pubkey') },
  { id: 'created_at', label: t('ln_address_list.created') },
  { id: 'updated_at', label: t('ln_address_list.updated') },
  { id: 'status', label: t('ln_address_list.status') },
  { id: 'allows_nostr', label: t('ln_address_list.allows_nostr') },
  { id: '' },
];

// ----------------------------------------------------------------------

export function LnAddressListView() {
  const theme = useTheme();
  const newLnAddress = useBoolean();
  const { t } = useTranslate();

  const { lnAddresses, lnAddressesLoading, lnAddressesError } = useListLnAddresses();

  const errors = [lnAddressesError];
  const data = [lnAddresses];
  const isLoading = [lnAddressesLoading];

  const failed = shouldFail(errors, data, isLoading);

  const tabs = [
    {
      title: t('ln_address_list.tabs.total'),
      value: 'all',
      label: t('ln_address_list.tabs.total'),
      color: 'default' as LabelColor,
      suffix: t('ln_address_list.tabs.addresses'),
      icon: 'solar:clipboard-list-bold-duotone',
      analyticColor: theme.palette.info.main,
    },
    {
      title: t('ln_address_list.tabs.active'),
      value: 'active',
      label: t('ln_address_list.tabs.active'),
      color: 'success' as LabelColor,
      suffix: t('ln_address_list.tabs.addresses'),
      icon: 'solar:check-square-bold-duotone',
      analyticColor: theme.palette.success.main,
    },
    {
      title: t('ln_address_list.tabs.inactive'),
      value: 'inactive',
      label: t('ln_address_list.tabs.inactive'),
      color: 'error' as LabelColor,
      suffix: t('ln_address_list.tabs.addresses'),
      icon: 'solar:forbidden-bold-duotone',
      analyticColor: theme.palette.error.main,
    },
  ];

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission['READ:LN_ADDRESS']]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('lightning_addresses')}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('lightning_addresses'),
                },
              ]}
              action={
                <Stack direction="row" spacing={1}>
                  <Button
                    onClick={newLnAddress.onTrue}
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

            <LnAddressList data={lnAddresses!} tableHead={tableHead(t)} tabs={tabs} />

            <RegisterLnAddressDialog
              open={newLnAddress.value}
              onClose={newLnAddress.onFalse}
              onSuccess={() => mutate(endpointKeys.lightning.addresses.list)}
              isAdmin
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
