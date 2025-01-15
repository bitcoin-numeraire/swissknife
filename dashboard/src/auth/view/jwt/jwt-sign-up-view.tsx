'use client';

import { z as zod } from 'zod';
import { useForm } from 'react-hook-form';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import LoadingButton from '@mui/lab/LoadingButton';
import InputAdornment from '@mui/material/InputAdornment';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { signUp } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';
import { Form, Field } from 'src/components/hook-form';

import { JWT_STORAGE_KEY } from 'src/auth/context/jwt';

import { useAuthContext } from '../../hooks';
import { FormHead } from '../../components/form-head';
import { SignUpTerms } from '../../components/sign-up-terms';

// ----------------------------------------------------------------------

export type SignUpSchemaType = zod.infer<typeof SignUpSchema>;

export const SignUpSchema = zod.object({
  password: zod.string().min(1).min(6),
  repeatPassword: zod.string().min(1).min(6),
});

// ----------------------------------------------------------------------

export function JwtSignUpView() {
  const router = useRouter();
  const { t } = useTranslate();

  const showPassword = useBoolean();

  const { checkUserSession } = useAuthContext();

  const defaultValues: SignUpSchemaType = {
    password: '',
    repeatPassword: '',
  };

  const methods = useForm<SignUpSchemaType>({
    resolver: zodResolver(SignUpSchema),
    defaultValues,
  });

  const {
    handleSubmit,
    formState: { isSubmitting },
  } = methods;

  const onSubmit = handleSubmit(async (body) => {
    try {
      const { data } = await signUp<true>({
        body: { password: body.password },
      });
      const accessToken = data.token;

      if (!accessToken) {
        throw new Error('Access token not found in response');
      }

      sessionStorage.setItem(JWT_STORAGE_KEY, accessToken);
      await checkUserSession?.();

      router.push(paths.wallet.root);
    } catch (error) {
      handleActionError(error);
    }
  });

  const renderForm = () => (
    <Box sx={{ gap: 3, display: 'flex', flexDirection: 'column' }}>
      <Field.Text
        name="password"
        label={t('sign_up.password')}
        type={showPassword.value ? 'text' : 'password'}
        slotProps={{
          inputLabel: { shrink: true },
          input: {
            endAdornment: (
              <InputAdornment position="end">
                <IconButton onClick={showPassword.onToggle} edge="end">
                  <Iconify icon={showPassword.value ? 'solar:eye-bold' : 'solar:eye-closed-bold'} />
                </IconButton>
              </InputAdornment>
            ),
          },
        }}
      />

      <Field.Text
        name="repeatPassword"
        label={t('sign_up.repeat_password')}
        type={showPassword.value ? 'text' : 'password'}
        slotProps={{
          inputLabel: { shrink: true },
          input: {
            endAdornment: (
              <InputAdornment position="end">
                <IconButton onClick={showPassword.onToggle} edge="end">
                  <Iconify icon={showPassword.value ? 'solar:eye-bold' : 'solar:eye-closed-bold'} />
                </IconButton>
              </InputAdornment>
            ),
          },
        }}
      />

      <LoadingButton
        fullWidth
        color="inherit"
        size="large"
        type="submit"
        variant="contained"
        loading={isSubmitting}
      >
        {t('sign_up.create_button')}
      </LoadingButton>
    </Box>
  );

  return (
    <>
      <FormHead
        title="Set up your Password"
        description="Your password is used to access your wallet and encrypt your data, you won't be able to log in if you lose but don't worry, your funds can be recovered."
        sx={{ textAlign: { xs: 'center', md: 'left' } }}
      />

      <Form methods={methods} onSubmit={onSubmit}>
        {renderForm()}
      </Form>

      <SignUpTerms />
    </>
  );
}
