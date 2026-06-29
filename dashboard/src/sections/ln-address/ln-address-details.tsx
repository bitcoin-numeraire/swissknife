import type { LnAddress } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useCallback } from 'react';
import { QRCode } from 'react-qrcode-logo';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Link from '@mui/material/Link';
import Stack from '@mui/material/Stack';
import Tooltip from '@mui/material/Tooltip';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';
import { RouterLink } from 'src/routes/components';

import { npub } from 'src/utils/nostr';
import { handleActionError } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';
import { encodeLNURL, displayLnAddress } from 'src/utils/lnurl';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { deleteAddress, deleteWalletAddress } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { DeleteButton } from 'src/components/delete';

import {
  DetailRow,
  DetailCard,
  MetricTile,
  formatDateTime,
  TransactionTimeline,
} from 'src/sections/transaction/transaction-detail-common';

// ----------------------------------------------------------------------

type Props = {
  lnAddress: LnAddress;
  isAdmin?: boolean;
};

export function LnAddressDetails({ lnAddress, isAdmin }: Props) {
  const { t } = useTranslate();
  const router = useRouter();
  const lnAddressDisplay = displayLnAddress(lnAddress.username);
  const encodedLnurl = encodeLNURL(lnAddress.username);
  const nostrDisplay = npub(lnAddress.nostr_pubkey);

  const onDelete = useCallback(
    async (id: string) => {
      try {
        if (isAdmin) {
          await deleteAddress({ path: { id } });
          router.push(paths.admin.lnAddresses);
        } else {
          await deleteWalletAddress();
        }

        mutate(endpointKeys.lightning.addresses.list);
        mutate(endpointKeys.userWallet.lnAddress.get);
        toast.success(`Lightning address deleted successfully: ${id}`);
      } catch (error) {
        handleActionError(error);
      }
    },
    [router, isAdmin]
  );

  return (
    <>
      <Stack
        spacing={3}
        direction={{ xs: 'column', sm: 'row' }}
        sx={{
          mb: { xs: 3, md: 5 },
          alignItems: { xs: 'flex-end', sm: 'center' },
        }}
      >
        <Stack direction="row" spacing={1} sx={{ width: 1, flexGrow: 1 }}>
          <CopyButton value={lnAddressDisplay} title={t('ln_address_details.copy')} />

          <Tooltip title={t('send')}>
            <IconButton onClick={() => toast.info(t('coming_soon'))}>
              <Iconify icon="iconamoon:send-fill" />
            </IconButton>
          </Tooltip>

          <Tooltip title={t('share')}>
            <IconButton onClick={() => toast.info(t('coming_soon'))}>
              <Iconify icon="solar:share-bold" />
            </IconButton>
          </Tooltip>

          <DeleteButton id={lnAddress.id} onDelete={onDelete} />

          {!isAdmin && (
            <Tooltip title={t('edit')}>
              <IconButton component={RouterLink} href={`${paths.identity}?tab=lightning`}>
                <Iconify icon="solar:pen-2-bold-duotone" />
              </IconButton>
            </Tooltip>
          )}
        </Stack>
      </Stack>

      <Stack spacing={3}>
        <Card sx={{ p: { xs: 3, md: 4 }, borderRadius: 1 }}>
          <Grid container spacing={4} sx={{ alignItems: 'stretch' }}>
            <Grid size={{ xs: 12, md: 7 }}>
              <Stack spacing={3} sx={{ height: 1 }}>
                <Stack spacing={1.25}>
                  <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
                    <Box
                      sx={{
                        width: 52,
                        height: 52,
                        display: 'grid',
                        borderRadius: 1,
                        placeItems: 'center',
                        color: 'success.contrastText',
                        bgcolor: 'success.main',
                      }}
                    >
                      <Iconify icon="solar:user-id-bold" width={30} />
                    </Box>

                    <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                      <Label variant="soft" color={lnAddress.active ? 'success' : 'error'}>
                        {lnAddress.active
                          ? t('ln_address_details.active')
                          : t('ln_address_details.deactivated')}
                      </Label>
                      <Label variant="soft" color={lnAddress.allows_nostr ? 'success' : 'error'}>
                        {lnAddress.allows_nostr
                          ? t('ln_address_details.nostr_visible')
                          : t('ln_address_details.nostr_deactivated')}
                      </Label>
                    </Stack>
                  </Stack>

                  <Stack spacing={0.75}>
                    <Typography variant="overline" color="text.secondary">
                      {t('ln_address_details.title')}
                    </Typography>
                    <Typography variant="h3" sx={{ fontWeight: 500, wordBreak: 'break-word' }}>
                      {lnAddressDisplay}
                    </Typography>
                    <Typography variant="body1" color="text.secondary">
                      {t('ln_address_details.subtitle')}
                    </Typography>
                  </Stack>
                </Stack>

                <Grid container spacing={2}>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile
                      title={t('ln_address_details.username')}
                      value={lnAddress.username}
                    />
                  </Grid>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile title={t('ln_address_details.domain')} value={CONFIG.domain} />
                  </Grid>
                  <Grid size={{ xs: 12, sm: 4 }}>
                    <MetricTile
                      title={t('ln_address_details.belongs_to')}
                      value={
                        isAdmin ? (
                          <Link
                            component={RouterLink}
                            href={paths.account(lnAddress.wallet_id)}
                            color="inherit"
                            underline="hover"
                            sx={{ wordBreak: 'break-word' }}
                          >
                            {truncateText(lnAddress.wallet_id, 15)}
                          </Link>
                        ) : (
                          truncateText(lnAddress.wallet_id, 15)
                        )
                      }
                    />
                  </Grid>
                </Grid>
              </Stack>
            </Grid>

            <Grid size={{ xs: 12, md: 5 }}>
              <Card variant="outlined" sx={{ p: 2, borderRadius: 1, height: 1 }}>
                <Stack spacing={2} sx={{ height: 1 }}>
                  <Stack
                    direction="row"
                    spacing={1}
                    sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                  >
                    <Typography variant="subtitle2">{t('ln_address_details.lnurl_pay')}</Typography>
                    <CopyButton value={encodedLnurl} title={t('copy')} />
                  </Stack>

                  <Box
                    sx={{
                      p: 1.5,
                      borderRadius: 1,
                      bgcolor: 'common.white',
                      display: 'grid',
                      placeItems: 'center',
                    }}
                  >
                    <QRCode
                      value={lnAddressDisplay}
                      size={260}
                      logoImage="/logo/logo_square_negative.svg"
                      removeQrCodeBehindLogo
                      logoPaddingStyle="circle"
                      eyeRadius={5}
                      logoPadding={3}
                    />
                  </Box>

                  <Stack
                    direction="row"
                    spacing={0.5}
                    sx={{ alignItems: 'center', minWidth: 0, color: 'text.secondary' }}
                  >
                    <Typography variant="body2" sx={{ minWidth: 0, wordBreak: 'break-word' }}>
                      {lnAddressDisplay}
                    </Typography>
                    <CopyButton value={lnAddressDisplay} title={t('copy')} />
                  </Stack>
                </Stack>
              </Card>
            </Grid>
          </Grid>
        </Card>

        <Grid container spacing={3}>
          <Grid size={{ xs: 12, md: 5 }}>
            <DetailCard
              title={t('transaction_details.timeline')}
              icon="solar:sort-by-time-bold-duotone"
              color="success"
            >
              <TransactionTimeline
                items={[
                  {
                    label: t('ln_address_details.creation_date'),
                    value: lnAddress.created_at,
                    state: 'done',
                  },
                  {
                    label: t('ln_address_details.update_date'),
                    value: lnAddress.updated_at,
                    state: lnAddress.updated_at ? 'done' : 'waiting',
                  },
                ]}
              />
            </DetailCard>
          </Grid>

          <Grid size={{ xs: 12, md: 7 }}>
            <DetailCard
              title={t('ln_address_details.context')}
              icon="solar:user-id-bold"
              color="success"
            >
              <DetailRow
                label={t('ln_address_details.username')}
                value={lnAddress.username}
                copyValue={lnAddress.username}
              />
              <DetailRow
                label={t('ln_address_details.title')}
                value={lnAddressDisplay}
                copyValue={lnAddressDisplay}
              />
              <DetailRow
                label={t('ln_address_details.belongs_to')}
                value={
                  isAdmin ? (
                    <Link
                      component={RouterLink}
                      href={paths.account(lnAddress.wallet_id)}
                      color="inherit"
                      underline="hover"
                    >
                      {lnAddress.wallet_id}
                    </Link>
                  ) : (
                    lnAddress.wallet_id
                  )
                }
                copyValue={lnAddress.wallet_id}
                mono
              />
              <DetailRow
                label={t('ln_address_list.status')}
                value={
                  lnAddress.active
                    ? t('ln_address_details.active')
                    : t('ln_address_details.deactivated')
                }
              />
              <DetailRow
                label={t('ln_address_list.allows_nostr')}
                value={
                  lnAddress.allows_nostr
                    ? t('ln_address_details.nostr_visible')
                    : t('ln_address_details.nostr_deactivated')
                }
              />
            </DetailCard>
          </Grid>

          <Grid size={{ xs: 12 }}>
            <DetailCard
              title={t('transaction_details.technical_details')}
              icon="solar:code-square-bold-duotone"
              color="success"
            >
              <DetailRow
                label={t('ln_address_details.address_id')}
                value={lnAddress.id}
                copyValue={lnAddress.id}
                mono
              />
              <DetailRow
                label={t('ln_address_details.wallet_id')}
                value={lnAddress.wallet_id}
                copyValue={lnAddress.wallet_id}
                mono
              />
              <DetailRow
                label={t('ln_address_details.lnurl')}
                value={encodedLnurl}
                copyValue={encodedLnurl}
                mono
              />
              {nostrDisplay && (
                <DetailRow
                  label={t('ln_address_list.nostr_pubkey')}
                  value={nostrDisplay}
                  copyValue={nostrDisplay}
                  mono
                />
              )}
              <DetailRow
                label={t('ln_address_details.creation_date')}
                value={formatDateTime(lnAddress.created_at)}
              />
              <DetailRow
                label={t('ln_address_details.update_date')}
                value={formatDateTime(lnAddress.updated_at)}
              />
            </DetailCard>
          </Grid>
        </Grid>
      </Stack>
    </>
  );
}
