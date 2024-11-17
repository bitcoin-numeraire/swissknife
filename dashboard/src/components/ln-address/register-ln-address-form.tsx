import type { RegisterLnAddressRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { ajvResolver } from '@hookform/resolvers/ajv';

import { Stack } from '@mui/material';
import { LoadingButton } from '@mui/lab';
import InputAdornment from '@mui/material/InputAdornment';

import { ajvOptions } from 'src/utils/ajv';

import { CONFIG } from 'src/config-global';
import { useTranslate } from 'src/locales';
import { registerAddress, registerWalletAddress, RegisterLnAddressRequestSchema } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Form, RHFTextField, RHFWalletSelect } from 'src/components/hook-form';

// ----------------------------------------------------------------------

type Props = {
  onSuccess: VoidFunction;
  isAdmin?: boolean;
};

// @ts-ignore
const resolver = ajvResolver(RegisterLnAddressRequestSchema, ajvOptions);

export function RegisterLnAddressForm({ onSuccess, isAdmin }: Props) {
  const { t } = useTranslate();

  const methods = useForm({
    resolver,
    defaultValues: {
      username: '',
      wallet: null,
    },
  });

  const {
    reset,
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const username = watch('username');
  const wallet = watch('wallet');

  const onSubmit = async (body: any) => {
    const submissionData: RegisterLnAddressRequest = {
      ...body,
      wallet_id: body.wallet?.id,
    };

    try {
      if (isAdmin) {
        await registerAddress({ body: submissionData });
      } else {
        await registerWalletAddress({ body: submissionData });
      }
      toast.success(t('register_ln_address.success_lightning_address_registration'));
      reset();
      onSuccess();
    } catch (error) {
      toast.error(error.reason);
    }
  };

  return (
    <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
      <Stack spacing={3}>
        <RHFTextField
          variant="outlined"
          name="username"
          label={t('ln_address_details.username')}
          onChange={(e) => {
            const value = e.target.value.toLowerCase();
            methods.setValue('username', value, { shouldValidate: true });
          }}
          InputProps={{
            endAdornment: <InputAdornment position="end">@{CONFIG.site.domain}</InputAdornment>,
          }}
        />

        {isAdmin && <RHFWalletSelect />}

        <LoadingButton
          type="submit"
          variant="contained"
          color="inherit"
          size="large"
          loading={isSubmitting}
          disabled={!username || isSubmitting || (isAdmin && !wallet)}
        >
          {t('register')}
        </LoadingButton>
      </Stack>
    </Form>
  );
}
