import { mutate } from 'swr';
import { useForm } from 'react-hook-form';
import { QRCode } from 'react-qrcode-logo';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import LoadingButton from '@mui/lab/LoadingButton';
import { Alert, Grid2, Typography, InputAdornment } from '@mui/material';

import { npub } from 'src/utils/nostr';
import { displayLnAddress } from 'src/utils/lnurl';
import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { zUpdateLnAddressRequest } from 'src/lib/swissknife/zod.gen';
import {
  type LnAddress,
  updateWalletAddress,
  deleteWalletAddress,
  type UpdateLnAddressRequest,
} from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Form, Field } from 'src/components/hook-form';
import { ConfirmDialog } from 'src/components/custom-dialog';

// ----------------------------------------------------------------------

type Props = {
  lnAddress: LnAddress;
};

export function SettingsLnAddress({ lnAddress }: Props) {
  const { t } = useTranslate();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  const defaultValues = {
    username: lnAddress.username || '',
    active: lnAddress.active || false,
    allows_nostr: lnAddress.allows_nostr || false,
    nostr_pubkey: npub(lnAddress.nostr_pubkey),
  };

  const methods = useForm({
    mode: 'all',
    resolver: zodResolver(zUpdateLnAddressRequest),
    defaultValues,
  });

  const {
    handleSubmit,
    formState: { isSubmitting },
  } = methods;

  const onSubmit = async (data: UpdateLnAddressRequest) => {
    // Map empty strings to undefined
    const body = Object.fromEntries(
      Object.entries(data).map(([key, value]) => [key, value === '' ? undefined : value])
    );

    try {
      await updateWalletAddress({ body });

      toast.success(t('settings_ln_address.update_success'));
      mutate(endpointKeys.userWallet.lnAddress.get);
    } catch (error) {
      handleActionError(error);
    }
  };

  const onDelete = async () => {
    isDeleting.onTrue();

    try {
      await deleteWalletAddress();

      toast.success(t('settings_ln_address.delete_success'));
      mutate(endpointKeys.userWallet.lnAddress.get);
    } catch (error) {
      handleActionError(error);
    } finally {
      isDeleting.onFalse();
    }
  };

  return (
    <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
      <Grid2 container spacing={3}>
        <Grid2 size={{ xs: 12, md: 4 }}>
          <Card
            sx={{
              pt: 5,
              pb: 5,
              px: 3,
              textAlign: 'center',
            }}
          >
            <Box
              sx={{
                height: 'auto',
                '& > canvas': {
                  width: '100% !important',
                  height: 'auto !important',
                },
              }}
            >
              <QRCode
                value={displayLnAddress(lnAddress!.username)}
                size={300} // Base size, will be overridden by CSS
                logoImage="/logo/logo_square_negative.svg"
                removeQrCodeBehindLogo
                logoPaddingStyle="circle"
                eyeRadius={5}
                logoPadding={3}
              />
              <Typography variant="subtitle1">{displayLnAddress(lnAddress!.username)}</Typography>
            </Box>

            <Field.Switch
              name="active"
              labelPlacement="start"
              label={t('settings_ln_address.active')}
              sx={{ mt: 2 }}
            />
            <Field.Switch
              name="allows_nostr"
              labelPlacement="start"
              label={t('settings_ln_address.nostr_visible')}
            />

            <Button variant="soft" color="error" sx={{ mt: 3 }} onClick={confirm.onTrue}>
              {t('settings_ln_address.delete_button')}
            </Button>
          </Card>
        </Grid2>

        <Grid2 size={{ xs: 12, md: 8 }}>
          <Card sx={{ p: { xs: 1, sm: 3 } }}>
            <Box
              rowGap={3}
              columnGap={2}
              display="grid"
              gridTemplateColumns={{ xs: 'repeat(1, 1fr)', sm: 'repeat(2, 1fr)' }}
            >
              <Field.Text
                name="username"
                label={t('settings_ln_address.username')}
                InputProps={{
                  endAdornment: <InputAdornment position="end">@{CONFIG.domain}</InputAdornment>,
                }}
              />
              <Field.Text
                name="nostr_pubkey"
                label={t('settings_ln_address.nostr_pubkey')}
                helperText={lnAddress.nostr_pubkey && `hex format: ${lnAddress.nostr_pubkey}`}
                sx={{ gridColumn: 'span 2' }}
              />
            </Box>

            <Stack spacing={3} alignItems="flex-end" sx={{ mt: 3 }}>
              <Alert variant="outlined" severity="warning" sx={{ width: '100%' }}>
                {t('settings_ln_address.alert')}
              </Alert>
              <LoadingButton type="submit" variant="contained" loading={isSubmitting}>
                {t('settings_ln_address.save')}
              </LoadingButton>
            </Stack>
          </Card>
        </Grid2>
      </Grid2>

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('delete')}
        content={t('confirm_delete')}
        action={
          <LoadingButton
            variant="contained"
            color="error"
            onClick={onDelete}
            loading={isDeleting.value}
          >
            {t('delete')}
          </LoadingButton>
        }
      />
    </Form>
  );
}
