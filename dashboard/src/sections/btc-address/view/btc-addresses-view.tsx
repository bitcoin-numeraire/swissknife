'use client';

import type { TFunction } from 'i18next';
import type { LabelColor } from 'src/components/label';
import type { Wallet, BtcAddress, BtcAddressType as BtcAddressTypeValue } from 'src/lib/swissknife';

import { useBoolean } from 'minimal-shared/hooks';
import { useMemo, useState, useEffect } from 'react';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import { useTheme } from '@mui/material/styles';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { shouldFail, handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useListWallets } from 'src/actions/wallet';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListBtcAddresses } from 'src/actions/btc-addresses';
import { Permission, BtcAddressType, generateBtcAddress } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
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

const addressTypeOptions = [
  {
    value: BtcAddressType.P2TR,
    labelKey: 'bitcoin_address_type.taproot',
    helperKey: 'bitcoin_address_type.taproot_helper',
  },
  {
    value: BtcAddressType.P2WPKH,
    labelKey: 'bitcoin_address_type.native_segwit',
    helperKey: 'bitcoin_address_type.native_segwit_helper',
  },
] as const;

const drawerSx = {
  width: { xs: 1, sm: 480 },
  maxWidth: 1,
};

function walletDisplayName(wallet: Wallet) {
  return wallet.ln_address?.username ?? wallet.user_id ?? wallet.id;
}

function compactId(id: string) {
  return id.length > 22 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;
}

function addressTypeLabel(t: TFunction, addressType: BtcAddressTypeValue) {
  const option = addressTypeOptions.find((item) => item.value === addressType);

  return option ? t(option.labelKey) : addressType.toUpperCase();
}

// ----------------------------------------------------------------------

export function BtcAddressesView() {
  const theme = useTheme();
  const { t } = useTranslate();
  const newAddress = useBoolean();
  const { btcAddresses, btcAddressesLoading, btcAddressesError, btcAddressesMutate } =
    useListBtcAddresses({
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
              action={
                <RoleBasedGuard
                  permissions={[Permission.READ_WALLET, Permission.WRITE_BTC_ADDRESS]}
                >
                  <Button
                    variant="contained"
                    startIcon={<Iconify icon="mingcute:add-line" />}
                    onClick={newAddress.onTrue}
                  >
                    {t('btc_address_list.new_address')}
                  </Button>
                </RoleBasedGuard>
              }
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <BtcAddressList data={btcAddresses!} tableHead={tableHead(t)} tabs={tabs} />

            <RoleBasedGuard permissions={[Permission.READ_WALLET, Permission.WRITE_BTC_ADDRESS]}>
              <GenerateBtcAddressDrawer
                open={newAddress.value}
                addresses={btcAddresses!}
                onClose={newAddress.onFalse}
                onGenerated={btcAddressesMutate}
              />
            </RoleBasedGuard>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}

type GenerateBtcAddressDrawerProps = {
  open: boolean;
  addresses: BtcAddress[];
  onClose: VoidFunction;
  onGenerated: () => Promise<unknown> | unknown;
};

function GenerateBtcAddressDrawer({
  open,
  addresses,
  onClose,
  onGenerated,
}: GenerateBtcAddressDrawerProps) {
  const { t } = useTranslate();
  const { wallets, walletsLoading, walletsError } = useListWallets();
  const [walletId, setWalletId] = useState('');
  const [addressType, setAddressType] = useState<BtcAddressTypeValue>(BtcAddressType.P2TR);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!open || !wallets?.length) return;

    if (!wallets.some((wallet) => wallet.id === walletId)) {
      setWalletId(wallets[0].id);
    }
  }, [open, walletId, wallets]);

  const selectedTypeOption = useMemo(
    () => addressTypeOptions.find((option) => option.value === addressType),
    [addressType]
  );

  const unusedAddressForType = useMemo(
    () =>
      addresses.find(
        (address) =>
          address.wallet_id === walletId && address.address_type === addressType && !address.used
      ),
    [addressType, addresses, walletId]
  );

  const handleGenerate = async () => {
    if (!walletId || unusedAddressForType) return;

    try {
      setIsSubmitting(true);
      await generateBtcAddress({ body: { wallet_id: walletId, type: addressType } });
      toast.success(t('btc_address_list.generate_success'));
      await onGenerated();
      onClose();
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const typeLabel = addressTypeLabel(t, addressType);

  return (
    <Drawer open={open} onClose={onClose} anchor="right" slotProps={{ paper: { sx: drawerSx } }}>
      <Stack spacing={3} sx={{ p: 3 }}>
        <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
          <Typography variant="h6">{t('btc_address_list.new_address')}</Typography>
          <IconButton onClick={onClose}>
            <Iconify icon="mingcute:close-line" />
          </IconButton>
        </Stack>

        <TextField
          select
          fullWidth
          label={t('wallet')}
          value={walletId}
          disabled={walletsLoading || !wallets?.length}
          helperText={t('btc_address_list.wallet_helper')}
          onChange={(event) => setWalletId(event.target.value)}
        >
          {wallets?.map((wallet) => (
            <MenuItem key={wallet.id} value={wallet.id}>
              <Stack spacing={0.25} sx={{ minWidth: 0 }}>
                <Typography noWrap variant="body2">
                  {walletDisplayName(wallet)}
                </Typography>
                <Typography noWrap variant="caption" color="text.secondary">
                  {compactId(wallet.id)}
                </Typography>
              </Stack>
            </MenuItem>
          ))}
        </TextField>

        <TextField
          select
          fullWidth
          label={t('receive_money.address_type')}
          value={addressType}
          helperText={t(selectedTypeOption?.helperKey ?? 'bitcoin_address_type.taproot_helper')}
          onChange={(event) => setAddressType(event.target.value as BtcAddressTypeValue)}
        >
          {addressTypeOptions.map((option) => (
            <MenuItem key={option.value} value={option.value}>
              {t(option.labelKey)}
            </MenuItem>
          ))}
        </TextField>

        {walletsError && <Alert severity="error">{t('btc_address_list.wallets_error')}</Alert>}

        {unusedAddressForType && (
          <Alert severity="info">
            {t('btc_address_list.unused_address_exists', { type: typeLabel })}
          </Alert>
        )}

        <Button
          size="large"
          variant="contained"
          loading={isSubmitting}
          disabled={!walletId || !!unusedAddressForType || walletsLoading}
          onClick={handleGenerate}
          startIcon={<Iconify icon="solar:refresh-bold" />}
        >
          {t('btc_address_list.generate_address')}
        </Button>
      </Stack>
    </Drawer>
  );
}
