'use client';

import { z as zod } from 'zod';
import { useMemo } from 'react';
import { useForm } from 'react-hook-form';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';
import InputAdornment from '@mui/material/InputAdornment';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { signUp } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';
import { Form, Field } from 'src/components/hook-form';
import { ONBOARDING_COMPLETE_STORAGE_KEY } from 'src/components/settings';

import { JWT_STORAGE_KEY } from 'src/auth/context/jwt';

import { useAuthContext } from '../../hooks';
import { FormHead } from '../../components/form-head';

// ----------------------------------------------------------------------

const MIN_PASSWORD_LENGTH = 12;

export type SignUpSchemaType = {
  password: string;
  repeatPassword: string;
};

// ----------------------------------------------------------------------

export function JwtSignUpView() {
  const router = useRouter();
  const { t } = useTranslate();

  const showPassword = useBoolean();

  const { checkUserSession } = useAuthContext();

  const signUpSchema = useMemo(
    () =>
      zod
        .object({
          password: zod.string().min(MIN_PASSWORD_LENGTH, {
            message: t('sign_up.min_password', { count: MIN_PASSWORD_LENGTH }),
          }),
          repeatPassword: zod.string().min(1, { message: t('sign_up.repeat_required') }),
        })
        .refine((data) => data.password === data.repeatPassword, {
          path: ['repeatPassword'],
          message: t('sign_up.passwords_do_not_match'),
        }),
    [t]
  );

  const defaultValues: SignUpSchemaType = {
    password: '',
    repeatPassword: '',
  };

  const methods = useForm<SignUpSchemaType>({
    resolver: zodResolver(signUpSchema),
    defaultValues,
  });

  const password = methods.watch('password');
  const passwordProgress = Math.min((password.length / MIN_PASSWORD_LENGTH) * 100, 100);

  const {
    handleSubmit,
    formState: { isSubmitting },
  } = methods;

  if (CONFIG.auth.method !== 'jwt') {
    return (
      <Stack spacing={3}>
        <FormHead
          title={t('sign_up.external_auth_title')}
          description={t('sign_up.external_auth_description')}
          sx={{ mb: 0, textAlign: 'left', alignItems: 'flex-start' }}
        />
        <Alert severity="info" sx={{ borderRadius: 1 }}>
          {t('sign_up.external_auth_note')}
        </Alert>
        <Button fullWidth color="inherit" size="large" variant="contained" href={paths.auth.login}>
          {t('sign_up.external_auth_action')}
        </Button>
      </Stack>
    );
  }

  const onSubmit = handleSubmit(async (body) => {
    try {
      const { data } = await signUp<true>({
        body: { password: body.password },
      });

      sessionStorage.setItem(JWT_STORAGE_KEY, data.token);
      localStorage.setItem(ONBOARDING_COMPLETE_STORAGE_KEY, 'true');
      await checkUserSession?.();

      router.replace(paths.wallet.root);
    } catch (error) {
      handleActionError(error);
    }
  });

  const renderForm = () => (
    <Box sx={{ gap: 2.5, display: 'flex', flexDirection: 'column' }}>
      <Field.Text
        name="password"
        label={t('sign_up.password')}
        helperText={t('sign_up.password_helper')}
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

      <Stack spacing={1}>
        <LinearProgress
          variant="determinate"
          value={passwordProgress}
          color={passwordProgress >= 100 ? 'success' : 'warning'}
          sx={{ height: 6, borderRadius: 1 }}
        />
        <Typography variant="caption" sx={{ color: 'text.secondary' }}>
          {t('sign_up.length_progress', {
            count: Math.min(password.length, MIN_PASSWORD_LENGTH),
            min: MIN_PASSWORD_LENGTH,
          })}
        </Typography>
      </Stack>

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

      <Button
        fullWidth
        color="inherit"
        size="large"
        type="submit"
        variant="contained"
        loading={isSubmitting}
        loadingIndicator={t('sign_up.create_loading')}
      >
        {t('sign_up.create_button')}
      </Button>
    </Box>
  );

  return (
    <Stack spacing={4}>
      <FormHead
        title={t('sign_up.title')}
        description={t('sign_up.description')}
        sx={{ mb: 0, textAlign: 'left', alignItems: 'flex-start' }}
      />

      <Form methods={methods} onSubmit={onSubmit}>
        {renderForm()}
      </Form>

      <Stack spacing={1.5}>
        <Typography variant="overline" sx={{ color: 'text.secondary', letterSpacing: 0 }}>
          {t('sign_up.security_title')}
        </Typography>

        {[
          ['solar:database-bold-duotone', 'sign_up.backup_node'],
          ['solar:shield-user-bold-duotone', 'sign_up.keep_private'],
          ['solar:bolt-bold-duotone', 'sign_up.fund_after_backup'],
        ].map(([icon, label]) => (
          <Stack key={label} direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
            <Iconify icon={icon} width={20} sx={{ color: 'primary.main' }} />
            <Typography variant="body2" sx={{ color: 'text.secondary' }}>
              {t(label)}
            </Typography>
          </Stack>
        ))}
      </Stack>
    </Stack>
  );
}
