'use client';

import type { SignInRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import LoadingButton from '@mui/lab/LoadingButton';
import InputAdornment from '@mui/material/InputAdornment';

import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { signIn } from 'src/lib/swissknife';
import { zSignInRequest } from 'src/lib/swissknife/zod.gen';

import { Iconify } from 'src/components/iconify';
import { Form, Field } from 'src/components/hook-form';
import { AnimateLogoRotate } from 'src/components/animate';

import { useAuthContext } from 'src/auth/hooks';
import { JWT_STORAGE_KEY } from 'src/auth/context/jwt';

import { FormHead } from '../../components/form-head';

// ----------------------------------------------------------------------

export function JwtSignInView() {
  const router = useRouter();

  const { t } = useTranslate();
  const { checkUserSession } = useAuthContext();

  const showPassword = useBoolean();

  const methods = useForm({
    resolver: zodResolver(zSignInRequest),
    defaultValues: {
      password: '',
    },
  });

  const {
    handleSubmit,
    formState: { isSubmitting },
  } = methods;

  const onSubmit = handleSubmit(async (body: SignInRequest) => {
    try {
      const { data } = await signIn<true>({ body });
      const accessToken = data.token;

      if (!accessToken) {
        throw new Error('Access token not found in response');
      }

      sessionStorage.setItem(JWT_STORAGE_KEY, accessToken);
      await checkUserSession?.();

      router.refresh();
    } catch (error) {
      handleActionError(error);
    }
  });

  const renderForm = () => (
    <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ gap: 1.5, display: 'flex', flexDirection: 'column' }}>
        <Field.Text
          name="password"
          label={t('sign_in.password')}
          type={showPassword.value ? 'text' : 'password'}
          slotProps={{
            inputLabel: { shrink: true },
            input: {
              endAdornment: (
                <InputAdornment position="end">
                  <IconButton onClick={showPassword.onToggle} edge="end">
                    <Iconify
                      icon={showPassword.value ? 'solar:eye-bold' : 'solar:eye-closed-bold'}
                    />
                  </IconButton>
                </InputAdornment>
              ),
            },
          }}
        />
      </Box>

      <LoadingButton
        fullWidth
        color="inherit"
        size="large"
        type="submit"
        variant="contained"
        loading={isSubmitting}
        loadingIndicator={t('sign_in.sign_in_loading')}
      >
        {t('sign_in.sign_in')}
      </LoadingButton>
    </Box>
  );

  return (
    <>
      <AnimateLogoRotate sx={{ mb: 3, mx: 'auto' }} />

      <FormHead title={t('sign_in.sign_in_to_your_account')} />

      <Form methods={methods} onSubmit={onSubmit}>
        {renderForm()}
      </Form>
    </>
  );
}
