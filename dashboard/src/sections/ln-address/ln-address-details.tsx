import type { LnAddress } from 'src/lib/swissknife';

import { mutate } from 'swr';
import Link from 'next/link';
import { useCallback } from 'react';
import { QRCode } from 'react-qrcode-logo';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { Grid, Tooltip, Divider, IconButton } from '@mui/material';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { npub } from 'src/utils/nostr';
import { fDate, fTime } from 'src/utils/format-time';
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

import { useAuthContext } from 'src/auth/hooks';

// ----------------------------------------------------------------------

type Props = {
  lnAddress: LnAddress;
  isAdmin?: boolean;
};

export function LnAddressDetails({ lnAddress, isAdmin }: Props) {
  const { t } = useTranslate();
  const { user } = useAuthContext();
  const router = useRouter();

  const lnAddressDisplay = displayLnAddress(lnAddress.username);

  const onDelete = useCallback(
    async (id: string) => {
      try {
        if (isAdmin) {
          await deleteAddress({ path: { id } });
          router.push(paths.admin.lnAddresses);
        } else {
          await deleteWalletAddress();
        }

        mutate(endpointKeys.userWallet.lnAddress.get);
      } catch (error) {
        handleActionError(error);
      } finally {
        toast.success(`Lightning address deleted successfully: ${id}`);
      }
    },
    [router, isAdmin]
  );

  const renderList = (
    <Grid container spacing={3} sx={{ my: 5 }}>
      <Grid item xs={12} sm={6}>
        <Title>{t('ln_address_details.username')}</Title>
        <Typography color="textSecondary">{lnAddress.username}</Typography>
      </Grid>
      <Grid item xs={12} sm={6}>
        <Title>{t('ln_address_details.domain')}</Title>
        <Typography color="textSecondary">{CONFIG.domain}</Typography>
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12} sm={6}>
        <Title>{t('ln_address_details.creation_date')}</Title>
        <Typography color="textSecondary">
          {fDate(lnAddress.created_at)} {fTime(lnAddress.created_at)}
        </Typography>
      </Grid>

      <Grid item xs={12} sm={6}>
        <Title>{t('ln_address_details.update_date')}</Title>
        <Typography color="textSecondary">
          {fDate(lnAddress.updated_at)} {fTime(lnAddress.updated_at)}
        </Typography>
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12}>
        <Title>LNURL</Title>
        <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
          {encodeLNURL(lnAddress.username)}
        </Typography>
      </Grid>

      <Grid item xs={12}>
        <Divider sx={{ borderStyle: 'dashed' }} />
      </Grid>

      <Grid item xs={12}>
        <Title>Nostr public Key</Title>
        <Typography color="textSecondary" sx={{ wordBreak: 'break-all' }}>
          {npub(lnAddress.nostr_pubkey)}
        </Typography>
      </Grid>
    </Grid>
  );

  return (
    <>
      <Stack
        spacing={3}
        direction={{ xs: 'column', sm: 'row' }}
        alignItems={{ xs: 'flex-end', sm: 'center' }}
        sx={{ mb: { xs: 3, md: 5 } }}
      >
        <Stack direction="row" spacing={1} flexGrow={1} sx={{ width: 1 }}>
          <CopyButton value={lnAddressDisplay} title={t('ln_address_details.copy')} />

          <Tooltip title={t('send')}>
            <IconButton
              onClick={() => {
                toast.info(t('coming_soon'));
              }}
            >
              <Iconify icon="iconamoon:send-fill" />
            </IconButton>
          </Tooltip>

          <Tooltip title={t('share')}>
            <IconButton
              onClick={() => {
                toast.info(t('coming_soon'));
              }}
            >
              <Iconify icon="solar:share-bold" />
            </IconButton>
          </Tooltip>

          <DeleteButton id={lnAddress.id} onDelete={onDelete} />

          <Tooltip title={t('edit')}>
            <Link href={{ pathname: paths.settings.root, query: { tab: 'lnaddress' } }}>
              <IconButton>
                <Iconify icon="solar:pen-2-bold-duotone" />
              </IconButton>
            </Link>
          </Tooltip>
        </Stack>
      </Stack>

      <Card
        sx={{ pt: 5, px: { xs: 2, sm: 5, md: 8 }, maxWidth: { xs: '100%', md: '80%' }, mx: 'auto' }}
      >
        <Box
          rowGap={5}
          display="grid"
          alignItems="center"
          gridTemplateColumns={{
            xs: 'repeat(1, 1fr)',
            sm: 'repeat(2, 1fr)',
          }}
        >
          <Typography variant="subtitle2">{lnAddress.id.toUpperCase()}</Typography>

          <Stack spacing={1} alignItems={{ xs: 'flex-start', md: 'flex-end' }}>
            <Stack direction="row" spacing={1}>
              <Label variant="soft" color={lnAddress.active ? 'success' : 'error'}>
                {lnAddress.active
                  ? t('ln_address_details.active')
                  : t('ln_address_details.deactivated')}
              </Label>
              <Label variant="soft" color={lnAddress.active ? 'success' : 'error'}>
                {lnAddress.allows_nostr
                  ? t('ln_address_details.nostr_visible')
                  : t('ln_address_details.nostr_deactivated')}
              </Label>
            </Stack>

            <Typography variant="subtitle1">{displayLnAddress(lnAddress.username)}</Typography>
          </Stack>

          <Stack sx={{ typography: 'body2' }}>
            <Typography fontWeight="bold" sx={{ mb: 1 }}>
              {t('ln_address_details.belongs_to')}
            </Typography>
            {isAdmin ? (
              truncateText(lnAddress.wallet_id, 15)
            ) : (
              <>
                {user?.displayName}
                <br />
                {user?.email}
                <br />
              </>
            )}
          </Stack>

          <Stack sx={{ typography: 'body2' }}>
            <Box
              sx={{
                width: '100%',
                maxWidth: 300,
                height: 'auto',
                '& > canvas': {
                  width: '100% !important',
                  height: 'auto !important',
                },
              }}
            >
              <QRCode
                value={lnAddressDisplay}
                size={300} // Base size, will be overridden by CSS
                logoImage="/logo/logo_square_negative.svg"
                removeQrCodeBehindLogo
                logoPaddingStyle="circle"
                eyeRadius={5}
                logoPadding={3}
              />
            </Box>
          </Stack>
        </Box>

        {renderList}
      </Card>
    </>
  );
}

type TitleProps = {
  children: React.ReactNode;
};

const Title = ({ children }: TitleProps): JSX.Element => (
  <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
    {children}
  </Typography>
);
