'use client';

import { mutate } from 'swr';
import { useMemo, useState } from 'react';
import { QRCode } from 'react-qrcode-logo';

import Tab from '@mui/material/Tab';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import LoadingButton from '@mui/lab/LoadingButton';

import { paths } from 'src/routes/paths';

import { npub } from 'src/utils/nostr';
import { shouldFail } from 'src/utils/errors';
import { fDateTime } from 'src/utils/format-time';
import { displayLnAddress } from 'src/utils/lnurl';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetUserWallet } from 'src/actions/user-wallet';
import { useListBtcAddresses } from 'src/actions/btc-addresses';
import { Permission , BtcAddressType, newWalletBtcAddress } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { EmptyContent } from 'src/components/empty-content';
import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

type IdentityTab = 'lightning' | 'nostr' | 'onchain' | 'people';

// ----------------------------------------------------------------------

export function IdentityView() {
  const { t } = useTranslate();
  const [tab, setTab] = useState<IdentityTab>('lightning');
  const [isGenerating, setIsGenerating] = useState(false);

  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const { btcAddresses, btcAddressesLoading, btcAddressesError, btcAddressesMutate } =
    useListBtcAddresses(wallet?.id ? { wallet_id: wallet.id } : undefined);

  const errors = [walletError];
  const data = [wallet];
  const isLoading = [walletLoading];
  const failed = shouldFail(errors, data, isLoading);

  const lnAddress = wallet?.ln_address;
  const displayAddress = lnAddress ? displayLnAddress(lnAddress.username) : '';
  const nostrDisplay = useMemo(() => npub(lnAddress?.nostr_pubkey), [lnAddress?.nostr_pubkey]);

  const handleGenerateAddress = async () => {
    try {
      setIsGenerating(true);
      await newWalletBtcAddress({ body: { type: BtcAddressType.P2TR } });
      btcAddressesMutate();
      mutate(endpointKeys.userWallet.get);
    } finally {
      setIsGenerating(false);
    }
  };

  return (
    <DashboardContent maxWidth="lg">
      <RoleBasedGuard permissions={[Permission.READ_LN_ADDRESS]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('identity_view.title')}
              links={[{ name: t('identity') }, { name: t('identity_hub') }]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

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
                <Tab value="people" label={t('identity_view.people')} />
              </Tabs>

              <Box sx={{ p: 3 }}>
                {tab === 'lightning' && (
                  <Stack direction={{ xs: 'column', md: 'row' }} spacing={3}>
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
                        value={displayAddress || ' '}
                        size={320}
                        eyeRadius={5}
                        logoPadding={3}
                        removeQrCodeBehindLogo
                        logoPaddingStyle="circle"
                        logoImage="/logo/logo_square_negative.svg"
                      />
                    </Box>

                    <Stack spacing={2} sx={{ flex: 1, minWidth: 0 }}>
                      <Stack spacing={0.5}>
                        <Typography variant="overline" color="text.secondary">
                          {t('identity_view.reusable_address')}
                        </Typography>
                        {displayAddress ? (
                          <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                            <Typography variant="h4" noWrap>
                              {displayAddress}
                            </Typography>
                            <CopyButton value={displayAddress} title={t('copy')} />
                          </Stack>
                        ) : (
                          <EmptyContent
                            title={t('identity_view.no_ln_address')}
                            action={
                              <Button href={paths.wallet.lightningAddress} variant="contained" color="inherit">
                                {t('register')}
                              </Button>
                            }
                          />
                        )}
                      </Stack>

                      {lnAddress && (
                        <Stack direction="row" spacing={1}>
                          <Label color={lnAddress.active ? 'success' : 'error'}>
                            {lnAddress.active ? t('ln_address_table_row.active') : t('ln_address_table_row.inactive')}
                          </Label>
                          <Label color={lnAddress.allows_nostr ? 'info' : 'default'}>NIP-05</Label>
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
                        <Alert severity={lnAddress?.allows_nostr ? 'success' : 'warning'} variant="outlined">
                          {lnAddress?.allows_nostr
                            ? t('identity_view.nostr_enabled')
                            : t('identity_view.nostr_disabled')}
                        </Alert>
                      </>
                    ) : (
                      <EmptyContent
                        title={t('identity_view.no_nostr')}
                        action={
                          <Button href={paths.wallet.nostrAddress} variant="contained" color="inherit">
                            {t('edit')}
                          </Button>
                        }
                      />
                    )}
                  </Stack>
                )}

                {tab === 'onchain' && (
                  <Stack spacing={2}>
                    <Stack
                      direction={{ xs: 'column', sm: 'row' }}
                      spacing={2}
                      sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}
                    >
                      <Stack>
                        <Typography variant="h6">{t('identity_view.onchain_addresses')}</Typography>
                        <Typography variant="body2" color="text.secondary">
                          {t('identity_view.onchain_subheader')}
                        </Typography>
                      </Stack>
                      <LoadingButton
                        color="inherit"
                        variant="contained"
                        loading={isGenerating}
                        onClick={handleGenerateAddress}
                        startIcon={<Iconify icon="solar:refresh-bold" />}
                      >
                        {t('identity_view.generate_fresh')}
                      </LoadingButton>
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
                          direction={{ xs: 'column', sm: 'row' }}
                          spacing={1}
                          sx={{ alignItems: { sm: 'center' }, py: 1 }}
                        >
                          <Stack sx={{ flex: 1, minWidth: 0 }}>
                            <Typography variant="subtitle2" noWrap>
                              {address.address}
                            </Typography>
                            <Typography variant="caption" color="text.secondary">
                              {address.address_type.toUpperCase()} · {fDateTime(address.created_at)}
                            </Typography>
                          </Stack>
                          <Label color={address.used ? 'warning' : 'success'}>
                            {address.used ? t('identity_view.used') : t('identity_view.unused')}
                          </Label>
                          <CopyButton value={address.address} title={t('copy')} />
                        </Stack>
                      ))}
                    </Stack>

                    {!btcAddressesLoading && !btcAddressesError && !btcAddresses?.length && (
                      <EmptyContent title={t('identity_view.no_onchain_addresses')} sx={{ py: 6 }} />
                    )}
                  </Stack>
                )}

                {tab === 'people' && (
                  <Stack spacing={2}>
                    {wallet?.contacts.length ? (
                      wallet.contacts.map((contact) => (
                        <Stack
                          key={contact.ln_address}
                          direction="row"
                          sx={{ alignItems: 'center', gap: 1.5 }}
                        >
                          <Box
                            sx={{
                              width: 36,
                              height: 36,
                              display: 'grid',
                              borderRadius: 1,
                              placeItems: 'center',
                              bgcolor: 'background.neutral',
                            }}
                          >
                            <Iconify icon="solar:user-rounded-bold" />
                          </Box>
                          <Typography variant="subtitle2" sx={{ flex: 1 }}>
                            {contact.ln_address}
                          </Typography>
                          <CopyButton value={contact.ln_address} title={t('copy')} />
                        </Stack>
                      ))
                    ) : (
                      <EmptyContent title={t('identity_view.no_people')} sx={{ py: 6 }} />
                    )}
                  </Stack>
                )}
              </Box>
            </Card>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
