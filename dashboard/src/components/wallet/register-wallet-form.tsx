import type { RegisterWalletRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import { Stack } from '@mui/material';
import { LoadingButton } from '@mui/lab';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { registerWallet } from 'src/lib/swissknife';
import { zRegisterWalletRequest } from 'src/lib/swissknife/zod.gen';

import { toast } from 'src/components/snackbar';
import { Form, RHFTextField } from 'src/components/hook-form';

// ----------------------------------------------------------------------

export type NewWalletFormProps = {
  onSuccess: VoidFunction;
};

export function RegisterWalletForm({ onSuccess }: NewWalletFormProps) {
  const { t } = useTranslate();

  const methods = useForm({
    resolver: zodResolver(zRegisterWalletRequest),
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
      handleActionError(error);
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
