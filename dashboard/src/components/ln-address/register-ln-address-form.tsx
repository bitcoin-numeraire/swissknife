import type { RegisterLnAddressRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import { Stack } from '@mui/material';
import { LoadingButton } from '@mui/lab';
import InputAdornment from '@mui/material/InputAdornment';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { zRegisterLnAddressRequest } from 'src/lib/swissknife/zod.gen';
import { registerAddress, registerWalletAddress } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Form, RHFTextField } from 'src/components/hook-form';

import { WalletSelectDropdown } from '../wallet';

// ----------------------------------------------------------------------

type Props = {
  onSuccess: VoidFunction;
  isAdmin?: boolean;
};

export function RegisterLnAddressForm({ onSuccess, isAdmin }: Props) {
  const { t } = useTranslate();

  const methods = useForm({
    resolver: zodResolver(zRegisterLnAddressRequest),
    defaultValues: {
      username: '',
      wallet_id: null,
    },
  });

  const {
    reset,
    handleSubmit,
    formState: { isSubmitting, isValid },
  } = methods;

  const onSubmit = async (body: RegisterLnAddressRequest) => {
    try {
      if (isAdmin) {
        await registerAddress({ body });
      } else {
        await registerWalletAddress({ body });
      }
      toast.success(t('register_ln_address.success_lightning_address_registration'));
      reset();
      onSuccess();
    } catch (error) {
      handleActionError(error);
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
            endAdornment: <InputAdornment position="end">@{CONFIG.domain}</InputAdornment>,
          }}
        />

        {isAdmin && <WalletSelectDropdown />}

        <LoadingButton
          type="submit"
          variant="contained"
          color="inherit"
          size="large"
          loading={isSubmitting}
          disabled={!isValid}
        >
          {t('register')}
        </LoadingButton>
      </Stack>
    </Form>
  );
}
