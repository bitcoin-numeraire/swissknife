'use client';

import type { SignInRequest } from 'src/lib/swissknife';

import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { ajvResolver } from '@hookform/resolvers/ajv';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import LoadingButton from '@mui/lab/LoadingButton';
import InputAdornment from '@mui/material/InputAdornment';

import { useRouter } from 'src/routes/hooks';

import { useBoolean } from 'src/hooks/use-boolean';

import { ajvOptions } from 'src/utils/ajv';

import { useTranslate } from 'src/locales';
import { signIn, SignInRequestSchema } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';
import { AnimateLogo2 } from 'src/components/animate';
import { Form, Field } from 'src/components/hook-form';

import { useAuthContext } from 'src/auth/hooks';
import { STORAGE_KEY } from 'src/auth/context/jwt';

// ----------------------------------------------------------------------

// @ts-ignore
const resolver = ajvResolver(SignInRequestSchema, ajvOptions);

export function JwtSignInView() {
  const router = useRouter();
  const { t } = useTranslate();

  const { checkUserSession } = useAuthContext();
  const [errorMsg, setErrorMsg] = useState('');
  const password = useBoolean();

  const methods = useForm({
    resolver,
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
      const { data } = await signIn({ body });
      const accessToken = data!.token;

      if (!accessToken) {
        throw new Error('Access token not found in response');
      }

      sessionStorage.setItem(STORAGE_KEY, accessToken);
      await checkUserSession?.();

      router.refresh();
    } catch (error) {
      setErrorMsg(error.reason);
    }
  });

  return (
    <>
      <AnimateLogo2 sx={{ mb: 3, mx: 'auto' }} />

      <Stack alignItems="center" spacing={1.5} sx={{ mb: 5 }}>
        <Typography variant="h5">{t('sign_in.sign_in_to_your_account')}</Typography>
      </Stack>

      {!!errorMsg && (
        <Alert severity="error" sx={{ mb: 3 }}>
          {errorMsg}
        </Alert>
      )}

      <Form methods={methods} onSubmit={onSubmit}>
        <Stack spacing={3}>
          <Field.Text
            name="password"
            label={t('sign_in.password')}
            type={password.value ? 'text' : 'password'}
            InputLabelProps={{ shrink: true }}
            InputProps={{
              endAdornment: (
                <InputAdornment position="end">
                  <IconButton onClick={password.onToggle} edge="end">
                    <Iconify icon={password.value ? 'solar:eye-bold' : 'solar:eye-closed-bold'} />
                  </IconButton>
                </InputAdornment>
              ),
            }}
          />

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
        </Stack>
      </Form>
    </>
  );
}
