import type { RegisterWalletRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { ajvResolver } from '@hookform/resolvers/ajv';

import { Stack } from '@mui/material';
import { LoadingButton } from '@mui/lab';

import { ajvOptions } from 'src/utils/ajv';

import { useTranslate } from 'src/locales';
import { registerWallet, RegisterWalletRequestSchema } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Form, RHFTextField } from 'src/components/hook-form';

// ----------------------------------------------------------------------

export type NewWalletFormProps = {
  onSuccess: VoidFunction;
};

// @ts-ignore
const resolver = ajvResolver(RegisterWalletRequestSchema, ajvOptions);

export function RegisterWalletForm({ onSuccess }: NewWalletFormProps) {
  const { t } = useTranslate();

  const methods = useForm({
    resolver,
    defaultValues: {
      user_id: '',
    },
  });

  const {
    reset,
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const user = watch('user_id');

  const onSubmit = async (body: RegisterWalletRequest) => {
    try {
      await registerWallet({ body });
      toast.success(t('register_wallet.success_wallet_registration'));
      reset();
      onSuccess();
    } catch (error) {
      toast.error(error.reason);
    }
  };

  return (
    <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
      <Stack spacing={3}>
        <RHFTextField variant="outlined" name="user_id" label="User" />

        <LoadingButton
          type="submit"
          variant="contained"
          color="inherit"
          size="large"
          loading={isSubmitting}
          disabled={!user || isSubmitting}
        >
          {t('register')}
        </LoadingButton>
      </Stack>
    </Form>
  );
}
