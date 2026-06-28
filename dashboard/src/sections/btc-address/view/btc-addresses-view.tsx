'use client';

import type { TFunction } from 'i18next';
import type { LabelColor } from 'src/components/label';

import { useTheme } from '@mui/material/styles';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListBtcAddresses } from 'src/actions/btc-addresses';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { BtcAddressList } from '../btc-address-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'address', label: t('btc_address_list.address') },
  { id: 'wallet_id', label: t('btc_address_list.wallet') },
  { id: 'address_type', label: t('btc_address_list.type') },
  { id: 'created_at', label: t('btc_address_list.created') },
  { id: 'updated_at', label: t('btc_address_list.updated') },
  { id: 'used', label: t('btc_address_list.status') },
  { id: '' },
];

// ----------------------------------------------------------------------

export function BtcAddressesView() {
  const theme = useTheme();
  const { t } = useTranslate();
  const { btcAddresses, btcAddressesLoading, btcAddressesError } = useListBtcAddresses({
    limit: 100,
  });

  const errors = [btcAddressesError];
  const data = [btcAddresses];
  const isLoading = [btcAddressesLoading];
  const failed = shouldFail(errors, data, isLoading);

  const tabs = [
    {
      title: t('btc_address_list.tabs.total'),
      value: 'all',
      label: t('btc_address_list.tabs.total'),
      color: 'default' as LabelColor,
      suffix: t('btc_address_list.tabs.addresses'),
      icon: 'solar:clipboard-list-bold-duotone',
      analyticColor: theme.palette.info.main,
    },
    {
      title: t('btc_address_list.tabs.unused'),
      value: 'unused',
      label: t('btc_address_list.tabs.unused'),
      color: 'success' as LabelColor,
      suffix: t('btc_address_list.tabs.addresses'),
      icon: 'solar:shield-check-bold-duotone',
      analyticColor: theme.palette.success.main,
    },
    {
      title: t('btc_address_list.tabs.used'),
      value: 'used',
      label: t('btc_address_list.tabs.used'),
      color: 'warning' as LabelColor,
      suffix: t('btc_address_list.tabs.addresses'),
      icon: 'solar:link-round-angle-bold-duotone',
      analyticColor: theme.palette.warning.main,
    },
  ];

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission.READ_BTC_ADDRESS]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('admin_bitcoin_addresses')}
              links={[{ name: t('accounts') }, { name: t('admin_bitcoin_addresses') }]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <BtcAddressList data={btcAddresses!} tableHead={tableHead(t)} tabs={tabs} />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
