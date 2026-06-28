'use client';

import type { BtcAddress } from 'src/lib/swissknife';

import { useMemo, useState } from 'react';

import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import InputAdornment from '@mui/material/InputAdornment';

import { paths } from 'src/routes/paths';

import { fDateTime } from 'src/utils/format-time';
import { shouldFail } from 'src/utils/errors';
import { compactBitcoinAddress } from 'src/utils/bitcoin-request';
import { bitcoinAddressExplorerUrl } from 'src/utils/bitcoin-explorer';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListBtcAddresses } from 'src/actions/btc-addresses';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

function addressTypeLabel(address: BtcAddress) {
  if (address.address_type === 'p2tr') return 'Taproot';
  if (address.address_type === 'p2wpkh') return 'Native SegWit';
  if (address.address_type === 'p2pkh') return 'Legacy';
  return address.address_type.toUpperCase();
}

function BtcAddressRow({ address }: { address: BtcAddress }) {
  const { t } = useTranslate();
  const explorerUrl = bitcoinAddressExplorerUrl(address.address);

  return (
    <Stack
      direction={{ xs: 'column', md: 'row' }}
      spacing={1.5}
      sx={{ alignItems: { md: 'center' }, py: 2 }}
    >
      <Stack spacing={0.5} sx={{ minWidth: 0, flex: 1 }}>
        <Stack
          direction="row"
          spacing={1}
          useFlexGap
          sx={{ alignItems: 'center', flexWrap: 'wrap' }}
        >
          <Typography
            variant="subtitle2"
            sx={{ fontFamily: 'monospace', minWidth: 0, wordBreak: 'break-word' }}
          >
            {compactBitcoinAddress(address.address)}
          </Typography>
          <CopyButton value={address.address} title={t('copy')} />
          {explorerUrl && (
            <IconButton
              component="a"
              href={explorerUrl}
              target="_blank"
              rel="noopener noreferrer"
              size="small"
            >
              <Iconify icon="solar:map-arrow-right-bold" width={18} />
            </IconButton>
          )}
        </Stack>
        <Typography variant="caption" color="text.secondary">
          {addressTypeLabel(address)} · {fDateTime(address.created_at)}
        </Typography>
      </Stack>

      <Stack direction="row" spacing={1} useFlexGap sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
        <Button
          color="inherit"
          href={paths.admin.wallet(address.wallet_id)}
          size="small"
          startIcon={<Iconify icon="solar:safe-square-bold-duotone" />}
        >
          {compactBitcoinAddress(address.wallet_id)}
        </Button>
        <Label variant="soft" color={address.used ? 'warning' : 'success'}>
          {address.used ? t('btc_address_list.used') : t('btc_address_list.unused')}
        </Label>
      </Stack>
    </Stack>
  );
}

export function BtcAddressesView() {
  const { t } = useTranslate();
  const [query, setQuery] = useState('');
  const { btcAddresses, btcAddressesLoading, btcAddressesError } = useListBtcAddresses({
    limit: 100,
  });

  const errors = [btcAddressesError];
  const data = [btcAddresses];
  const isLoading = [btcAddressesLoading];
  const failed = shouldFail(errors, data, isLoading);
  const addresses = btcAddresses ?? [];

  const filteredAddresses = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();

    if (!normalizedQuery) return addresses;

    return addresses.filter((address) =>
      [address.address, address.wallet_id, address.address_type, address.used ? 'used' : 'unused']
        .join(' ')
        .toLowerCase()
        .includes(normalizedQuery)
    );
  }, [addresses, query]);

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_BTC_ADDRESS]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('admin_bitcoin_addresses')}
              links={[{ name: t('admin') }, { name: t('admin_bitcoin_addresses') }]}
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Card sx={{ p: 3, borderRadius: 1 }}>
              <Stack spacing={2.5}>
                <TextField
                  fullWidth
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder={t('btc_address_list.search_placeholder')}
                  InputProps={{
                    startAdornment: (
                      <InputAdornment position="start">
                        <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled' }} />
                      </InputAdornment>
                    ),
                  }}
                />

                <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                  <Label variant="soft" color="info">
                    {filteredAddresses.length} / {addresses.length}
                  </Label>
                  <Typography variant="body2" color="text.secondary">
                    {t('btc_address_list.visible_count')}
                  </Typography>
                </Stack>

                {filteredAddresses.length ? (
                  <Stack divider={<Divider flexItem sx={{ borderStyle: 'dashed' }} />}>
                    {filteredAddresses.map((address) => (
                      <BtcAddressRow key={address.id} address={address} />
                    ))}
                  </Stack>
                ) : (
                  <EmptyContent title={t('btc_address_list.empty_title')} sx={{ py: 6 }} />
                )}
              </Stack>
            </Card>
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
