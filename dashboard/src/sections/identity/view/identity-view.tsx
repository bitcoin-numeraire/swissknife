'use client';

import type { ReactNode } from 'react';

import { mutate } from 'swr';
import { useMemo, useState } from 'react';
import { QRCode } from 'react-qrcode-logo';
import { useBoolean, useCopyToClipboard } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Grid from '@mui/material/Grid';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';

import { useSearchParams } from 'src/routes/hooks';

import { npub } from 'src/utils/nostr';
import { fDateTime } from 'src/utils/format-time';
import { encodeLNURL, displayLnAddress } from 'src/utils/lnurl';
import { shouldFail, handleActionError } from 'src/utils/errors';
import { compactBitcoinAddress } from 'src/utils/bitcoin-request';
import { bitcoinAddressExplorerUrl } from 'src/utils/bitcoin-explorer';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { BtcAddressType, newWalletBtcAddress } from 'src/lib/swissknife';
import { useGetUserWallet, useListWalletBtcAddresses } from 'src/actions/user-wallet';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { EmptyContent } from 'src/components/empty-content';
import { ErrorView } from 'src/components/error/error-view';
import { RegisterLnAddressForm } from 'src/components/ln-address';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { SettingsLnAddress } from 'src/sections/settings/settings-ln-address';

// ----------------------------------------------------------------------

type IdentityTab = 'lightning' | 'nostr' | 'onchain';

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
  width: { xs: 1, md: 760 },
  maxWidth: 1,
};

function BitcoinAddressValue({ address }: { address: string }) {
  const { t } = useTranslate();
  const explorerUrl = bitcoinAddressExplorerUrl(address);

  return (
    <Stack direction="row" spacing={0.5} sx={{ alignItems: 'center', minWidth: 0 }}>
      <Typography
        variant="subtitle2"
        noWrap
        sx={{ minWidth: 0, fontFamily: 'monospace', color: 'text.primary' }}
      >
        {compactBitcoinAddress(address)}
      </Typography>
      <CopyButton value={address} title={t('copy')} />
      {explorerUrl && (
        <IconButton
          component="a"
          href={explorerUrl}
          target="_blank"
          rel="noopener noreferrer"
          title={t('transaction_actions.open_explorer')}
        >
          <Iconify icon="solar:map-arrow-right-bold" />
        </IconButton>
      )}
    </Stack>
  );
}

// ----------------------------------------------------------------------

export function IdentityView() {
  const { t } = useTranslate();
  const { copy } = useCopyToClipboard();
  const searchParams = useSearchParams();
  const requestedTab = searchParams.get('tab');
  const [tab, setTab] = useState<IdentityTab>(
    requestedTab === 'nostr' || requestedTab === 'onchain' || requestedTab === 'lightning'
      ? requestedTab
      : 'lightning'
  );
  const [isGenerating, setIsGenerating] = useState(false);
  const [addressType, setAddressType] = useState<BtcAddressType>(BtcAddressType.P2TR);
  const registerDrawer = useBoolean();
  const manageDrawer = useBoolean();

  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const { btcAddresses, btcAddressesLoading, btcAddressesError, btcAddressesMutate } =
    useListWalletBtcAddresses();

  const errors = [walletError];
  const data = [wallet];
  const isLoading = [walletLoading];
  const failed = shouldFail(errors, data, isLoading);

  const lnAddress = wallet?.ln_address;
  const displayAddress = lnAddress ? displayLnAddress(lnAddress.username) : '';
  const identityLnurl = lnAddress ? encodeLNURL(lnAddress.username) : '';
  const nostrDisplay = useMemo(() => npub(lnAddress?.nostr_pubkey), [lnAddress?.nostr_pubkey]);
  const unusedAddressForType = useMemo(
    () => btcAddresses?.find((address) => address.address_type === addressType && !address.used),
    [addressType, btcAddresses]
  );

  const handleGenerateAddress = async () => {
    if (!wallet?.id) return;

    try {
      setIsGenerating(true);
      await newWalletBtcAddress({
        path: { wallet_id: wallet.id },
        body: { type: addressType },
      });
      btcAddressesMutate();
      mutate(endpointKeys.userWallet.get);
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsGenerating(false);
    }
  };

  const handleCopyUnusedAddress = () => {
    if (!unusedAddressForType) return;

    copy(unusedAddressForType.address);
    toast.success(t('copied_to_clipboard'));
  };

  const handleCopyIdentityLnurl = () => {
    if (!identityLnurl) return;

    copy(identityLnurl);
    toast.success(t('copied_to_clipboard'));
  };

  const handleIdentityChanged = () => {
    mutate(endpointKeys.userWallet.get);
    mutate(endpointKeys.userWallet.lnAddress.get);
  };

  const handleRegisterSuccess = () => {
    handleIdentityChanged();
    registerDrawer.onFalse();
  };

  return (
    <DashboardContent maxWidth="lg">
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('identity_view.title')}
            links={[{ name: t('identity') }, { name: t('identity_hub') }]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <Card sx={{ p: 3, mb: 3, borderRadius: 1 }}>
            <Grid container spacing={3} sx={{ alignItems: 'stretch' }}>
              <Grid size={{ xs: 12, md: 5 }}>
                <Stack spacing={2} sx={{ height: 1, justifyContent: 'center' }}>
                  <Box
                    sx={{
                      width: 56,
                      height: 56,
                      display: 'grid',
                      borderRadius: 1,
                      placeItems: 'center',
                      color: 'warning.contrastText',
                      bgcolor: 'warning.main',
                    }}
                  >
                    <Iconify icon="solar:user-rounded-bold-duotone" width={32} />
                  </Box>
                  <Stack spacing={1}>
                    <Typography variant="h4">{t('identity_view.hero_title')}</Typography>
                    <Typography variant="body2" color="text.secondary">
                      {t('identity_view.hero_subheader')}
                    </Typography>
                  </Stack>
                  {!displayAddress && (
                    <Button
                      onClick={registerDrawer.onTrue}
                      variant="contained"
                      color="inherit"
                      startIcon={<Iconify icon="solar:bolt-bold-duotone" />}
                      sx={{ alignSelf: 'flex-start' }}
                    >
                      {t('identity_view.claim_lightning')}
                    </Button>
                  )}
                </Stack>
              </Grid>

              <Grid size={{ xs: 12, md: 7 }}>
                <Grid container spacing={2} sx={{ height: 1 }}>
                  {[
                    {
                      title: t('identity_view.lightning'),
                      body: displayAddress
                        ? t('identity_view.lightning_ready')
                        : t('identity_view.lightning_body'),
                      icon: 'solar:bolt-bold-duotone',
                      color: 'warning',
                    },
                    {
                      title: t('identity_view.nostr'),
                      body: nostrDisplay
                        ? t('identity_view.nostr_ready')
                        : t('identity_view.nostr_body'),
                      icon: 'solar:verified-check-bold-duotone',
                      color: 'info',
                    },
                    {
                      title: t('identity_view.onchain'),
                      body: t('identity_view.onchain_body'),
                      icon: 'solar:qr-code-bold-duotone',
                      color: 'success',
                    },
                  ].map((item) => (
                    <Grid key={item.title} size={{ xs: 12, sm: 4 }}>
                      <Box
                        sx={[
                          (theme) => ({
                            p: 2,
                            height: 1,
                            borderRadius: 1,
                            bgcolor: 'background.neutral',
                            border: `1px solid ${theme.vars.palette.divider}`,
                          }),
                        ]}
                      >
                        <Stack spacing={1.5}>
                          <Box
                            sx={{
                              width: 36,
                              height: 36,
                              display: 'grid',
                              borderRadius: 1,
                              placeItems: 'center',
                              color: `${item.color}.contrastText`,
                              bgcolor: `${item.color}.main`,
                            }}
                          >
                            <Iconify icon={item.icon} width={22} />
                          </Box>
                          <Typography variant="subtitle2">{item.title}</Typography>
                          <Typography variant="body2" color="text.secondary">
                            {item.body}
                          </Typography>
                        </Stack>
                      </Box>
                    </Grid>
                  ))}
                </Grid>
              </Grid>
            </Grid>
          </Card>

          <Card sx={{ borderRadius: 1 }}>
            <Tabs
              value={tab}
              onChange={(_, value) => setTab(value)}
              variant="scrollable"
              sx={{ px: 2, borderBottom: (theme) => `1px solid ${theme.vars.palette.divider}` }}
            >
              <Tab value="lightning" label={t('identity_view.lightning')} />
              <Tab value="nostr" label={t('identity_view.nostr')} />
              <Tab value="onchain" label={t('identity_view.onchain')} />
            </Tabs>

            <Box sx={{ p: 3 }}>
              {tab === 'lightning' && (
                <Stack direction={{ xs: 'column', md: 'row' }} spacing={3}>
                  {displayAddress && (
                    <Box
                      sx={{
                        p: 2,
                        width: { xs: 1, md: 320 },
                        borderRadius: 1,
                        bgcolor: 'common.white',
                        '& canvas': { width: '100% !important', height: 'auto !important' },
                      }}
                    >
                      <QRCode
                        value={identityLnurl}
                        size={320}
                        eyeRadius={5}
                        logoPadding={3}
                        removeQrCodeBehindLogo
                        logoPaddingStyle="circle"
                        logoImage="/logo/logo_square_negative.svg"
                      />
                    </Box>
                  )}

                  <Stack spacing={2} sx={{ flex: 1, minWidth: 0 }}>
                    <Stack spacing={0.5}>
                      <Typography variant="overline" color="text.secondary">
                        {t('identity_view.reusable_address')}
                      </Typography>
                      {displayAddress ? (
                        <>
                          <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                            <Typography variant="h4" noWrap>
                              {displayAddress}
                            </Typography>
                            <CopyButton value={displayAddress} title={t('copy')} />
                          </Stack>
                          {identityLnurl && (
                            <Button
                              color="inherit"
                              variant="outlined"
                              onClick={handleCopyIdentityLnurl}
                              startIcon={<Iconify icon="solar:copy-bold" />}
                              sx={{ alignSelf: 'flex-start' }}
                            >
                              {t('identity_view.copy_lnurl')}
                            </Button>
                          )}
                        </>
                      ) : (
                        <Box
                          sx={[
                            (theme) => ({
                              p: 3,
                              borderRadius: 1,
                              bgcolor: 'background.neutral',
                              border: `1px dashed ${theme.vars.palette.divider}`,
                            }),
                          ]}
                        >
                          <Stack spacing={2}>
                            <Iconify
                              icon="solar:letter-bold-duotone"
                              width={46}
                              sx={{ color: 'text.disabled' }}
                            />
                            <Stack spacing={0.5}>
                              <Typography variant="h6">
                                {t('identity_view.no_ln_address')}
                              </Typography>
                              <Typography variant="body2" color="text.secondary">
                                {t('identity_view.no_ln_address_description')}
                              </Typography>
                            </Stack>
                            <Button
                              onClick={registerDrawer.onTrue}
                              variant="contained"
                              color="inherit"
                              sx={{ alignSelf: 'flex-start' }}
                            >
                              {t('register')}
                            </Button>
                          </Stack>
                        </Box>
                      )}
                    </Stack>

                    {lnAddress && (
                      <Stack spacing={1.5}>
                        <Stack direction="row" spacing={1}>
                          <Label color={lnAddress.active ? 'success' : 'error'}>
                            {lnAddress.active
                              ? t('ln_address_table_row.active')
                              : t('ln_address_table_row.inactive')}
                          </Label>
                          <Label color={lnAddress.allows_nostr ? 'info' : 'default'}>NIP-05</Label>
                        </Stack>
                        <Button
                          color="inherit"
                          variant="outlined"
                          onClick={manageDrawer.onTrue}
                          startIcon={<Iconify icon="solar:pen-bold" />}
                          sx={{ alignSelf: 'flex-start' }}
                        >
                          {t('identity_view.manage_identity')}
                        </Button>
                      </Stack>
                    )}
                  </Stack>
                </Stack>
              )}

              {tab === 'nostr' && (
                <Stack spacing={2}>
                  {nostrDisplay ? (
                    <>
                      <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                        <Typography variant="h5" noWrap>
                          {nostrDisplay}
                        </Typography>
                        <CopyButton value={nostrDisplay} title={t('copy')} />
                      </Stack>
                      <Alert
                        severity={lnAddress?.allows_nostr ? 'success' : 'warning'}
                        variant="outlined"
                      >
                        {lnAddress?.allows_nostr
                          ? t('identity_view.nostr_enabled')
                          : t('identity_view.nostr_disabled')}
                      </Alert>
                      <Button
                        color="inherit"
                        variant="outlined"
                        onClick={manageDrawer.onTrue}
                        startIcon={<Iconify icon="solar:pen-bold" />}
                        sx={{ alignSelf: 'flex-start' }}
                      >
                        {t('identity_view.edit_nostr')}
                      </Button>
                    </>
                  ) : (
                    <EmptyContent
                      title={t('identity_view.no_nostr')}
                      action={
                        <Button
                          onClick={lnAddress ? manageDrawer.onTrue : registerDrawer.onTrue}
                          variant="contained"
                          color="inherit"
                        >
                          {lnAddress
                            ? t('identity_view.edit_nostr')
                            : t('identity_view.claim_lightning')}
                        </Button>
                      }
                    />
                  )}
                </Stack>
              )}

              {tab === 'onchain' && (
                <Stack spacing={2}>
                  <Stack spacing={2}>
                    <Stack>
                      <Typography variant="h6">{t('identity_view.onchain_addresses')}</Typography>
                      <Typography variant="body2" color="text.secondary">
                        {t('identity_view.onchain_subheader')}
                      </Typography>
                    </Stack>
                    <Stack
                      direction={{ xs: 'column', sm: 'row' }}
                      spacing={2}
                      sx={{ alignItems: { sm: 'flex-start' } }}
                    >
                      <TextField
                        select
                        size="small"
                        label={t('receive_money.address_type')}
                        value={addressType}
                        onChange={(event) => setAddressType(event.target.value as BtcAddressType)}
                        helperText={t(
                          unusedAddressForType
                            ? 'identity_view.unused_address_available'
                            : (addressTypeOptions.find((option) => option.value === addressType)
                                ?.helperKey ?? 'bitcoin_address_type.taproot_helper')
                        )}
                        sx={{ minWidth: { sm: 260 } }}
                      >
                        {addressTypeOptions.map((option) => (
                          <MenuItem key={option.value} value={option.value}>
                            {t(option.labelKey)}
                          </MenuItem>
                        ))}
                      </TextField>
                      {unusedAddressForType ? (
                        <Button
                          color="inherit"
                          variant="outlined"
                          onClick={handleCopyUnusedAddress}
                          startIcon={<Iconify icon="solar:copy-bold" />}
                          sx={{ minHeight: 40 }}
                        >
                          {t('identity_view.copy_unused_address')}
                        </Button>
                      ) : (
                        <Button
                          color="inherit"
                          variant="contained"
                          loading={isGenerating}
                          onClick={handleGenerateAddress}
                          startIcon={<Iconify icon="solar:refresh-bold" />}
                          sx={{ minHeight: 40 }}
                        >
                          {t('identity_view.generate_fresh')}
                        </Button>
                      )}
                    </Stack>
                  </Stack>

                  {btcAddressesError && (
                    <Alert severity="warning" variant="outlined">
                      {t('identity_view.btc_addresses_unavailable')}
                    </Alert>
                  )}

                  {btcAddressesLoading && (
                    <Alert severity="info" variant="outlined">
                      {t('identity_view.loading_addresses')}
                    </Alert>
                  )}

                  <Stack spacing={1.5} divider={<Divider flexItem />}>
                    {(btcAddresses ?? []).map((address) => (
                      <Stack
                        key={address.id}
                        direction="row"
                        spacing={1}
                        sx={{ alignItems: 'flex-start', minWidth: 0, py: 1 }}
                      >
                        <Stack sx={{ flex: 1, minWidth: 0 }}>
                          <BitcoinAddressValue address={address.address} />
                          <Typography variant="caption" color="text.secondary">
                            {address.address_type.toUpperCase()} · {fDateTime(address.created_at)}
                          </Typography>
                        </Stack>
                        <Label
                          color={address.used ? 'warning' : 'success'}
                          sx={{ flexShrink: 0, mt: 0.75 }}
                        >
                          {address.used ? t('identity_view.used') : t('identity_view.unused')}
                        </Label>
                      </Stack>
                    ))}
                  </Stack>

                  {!btcAddressesLoading && !btcAddressesError && !btcAddresses?.length && (
                    <EmptyContent title={t('identity_view.no_onchain_addresses')} sx={{ py: 6 }} />
                  )}
                </Stack>
              )}
            </Box>
          </Card>

          <IdentityDrawer
            open={registerDrawer.value}
            title={t('identity_view.claim_lightning')}
            onClose={registerDrawer.onFalse}
          >
            <Stack spacing={3}>
              <Alert severity="info" variant="outlined">
                {t('identity_view.claim_lightning_description')}
              </Alert>
              <RegisterLnAddressForm onSuccess={handleRegisterSuccess} />
            </Stack>
          </IdentityDrawer>

          <IdentityDrawer
            open={manageDrawer.value}
            title={t('identity_view.manage_identity')}
            onClose={manageDrawer.onFalse}
          >
            {lnAddress ? (
              <SettingsLnAddress lnAddress={lnAddress} onSuccess={handleIdentityChanged} />
            ) : (
              <RegisterLnAddressForm onSuccess={handleRegisterSuccess} />
            )}
          </IdentityDrawer>
        </>
      )}
    </DashboardContent>
  );
}

function IdentityDrawer({
  open,
  title,
  children,
  onClose,
}: {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: VoidFunction;
}) {
  return (
    <Drawer anchor="right" open={open} onClose={onClose} slotProps={{ paper: { sx: drawerSx } }}>
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
      >
        <Typography variant="h6">{title}</Typography>
        <IconButton onClick={onClose}>
          <Iconify icon="mingcute:close-line" />
        </IconButton>
      </Stack>
      <Divider />
      <Box sx={{ p: 3 }}>{children}</Box>
    </Drawer>
  );
}
