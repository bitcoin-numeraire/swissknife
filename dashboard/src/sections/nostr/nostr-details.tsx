import { mutate } from 'swr';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import LoadingButton from '@mui/lab/LoadingButton';
import { Card, Alert, Stack } from '@mui/material';

import { npub } from 'src/utils/nostr';
import { displayLnAddress } from 'src/utils/lnurl';
import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { zUpdateLnAddressRequest } from 'src/lib/swissknife/zod.gen';
import {
  type LnAddress,
  updateWalletAddress,
  type UpdateLnAddressRequest,
} from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Form, RHFSwitch, RHFTextField } from 'src/components/hook-form';

// ----------------------------------------------------------------------

type Props = {
  lnAddress: LnAddress;
};

export function NostrDetails({ lnAddress }: Props) {
  const { t } = useTranslate();

  const defaultValues = {
    nostr_pubkey: npub(lnAddress.nostr_pubkey),
    allows_nostr: lnAddress.allows_nostr || false,
  };

  const methods = useForm({
    mode: 'all',
    resolver: zodResolver(zUpdateLnAddressRequest),
    defaultValues,
  });

  const {
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const nostrPubkey = watch('nostr_pubkey');

  const onSubmit = async (body: UpdateLnAddressRequest) => {
    try {
      await updateWalletAddress({
        body: {
          nostr_pubkey: body.nostr_pubkey || undefined,
          allows_nostr: body.allows_nostr,
        },
      });

      toast.success('Nostr Address updated successfully');
      mutate(endpointKeys.userWallet.lnAddress.get);
    } catch (error) {
      handleActionError(error);
    }
  };

  return (
    <Card sx={{ p: { xs: 1, sm: 3 }, maxWidth: { xs: '100%', md: '80%' }, mx: 'auto' }}>
      <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
        <Alert variant="outlined" severity="info" sx={{ mb: 4 }}>
          {t('nostr_details.alert', { lnAddress: displayLnAddress(lnAddress.username) })}
        </Alert>

        <Stack spacing={3}>
          <RHFTextField
            variant="outlined"
            name="nostr_pubkey"
            label={t('nostr_details.nostr_pubkey')}
            helperText={lnAddress.nostr_pubkey && `hex format: ${lnAddress.nostr_pubkey}`}
          />

          <RHFSwitch name="allows_nostr" labelPlacement="start" label="Visible on Nostr" />

          <LoadingButton
            type="submit"
            variant="contained"
            color="inherit"
            size="large"
            loading={isSubmitting}
            disabled={!nostrPubkey || isSubmitting}
          >
            {t('nostr_details.register_button')}
          </LoadingButton>
        </Stack>
      </Form>
    </Card>
  );
}
