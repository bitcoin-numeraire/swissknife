'use client';

import type { SignInRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import Link from '@mui/material/Link';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import InputAdornment from '@mui/material/InputAdornment';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';
import { RouterLink } from 'src/routes/components';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { signIn } from 'src/lib/swissknife';
import { zSignInRequest } from 'src/lib/swissknife/zod.gen';

import { Iconify } from 'src/components/iconify';
import { Form, Field } from 'src/components/hook-form';

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
    <Box sx={{ gap: 2.5, display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ gap: 1.5, display: 'flex', flexDirection: 'column' }}>
        <Field.Text
          name="password"
          label={t('sign_in.password')}
          helperText={t('sign_in.password_helper')}
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

      <Button
        fullWidth
        color="inherit"
        size="large"
        type="submit"
        variant="contained"
        loading={isSubmitting}
        loadingIndicator={t('sign_in.sign_in_loading')}
      >
        {t('sign_in.sign_in')}
      </Button>
    </Box>
  );

  return (
    <Stack spacing={4}>
      <FormHead
        title={t('sign_in.title')}
        description={t('sign_in.description')}
        sx={{ mb: 0, textAlign: 'left', alignItems: 'flex-start' }}
      />

      <Alert
        severity="info"
        icon={<Iconify icon="solar:shield-keyhole-bold-duotone" />}
        sx={{ borderRadius: 1 }}
      >
        {t('sign_in.local_password_note')}
      </Alert>

      <Form methods={methods} onSubmit={onSubmit}>
        {renderForm()}
      </Form>

      <Stack spacing={1.5}>
        <Typography variant="overline" sx={{ color: 'text.secondary', letterSpacing: 0 }}>
          {t('sign_in.vault_status')}
        </Typography>

        {[
          ['solar:monitor-smartphone-bold-duotone', 'sign_in.session_status'],
          ['solar:wallet-money-bold-duotone', 'sign_in.wallet_status'],
          ['solar:user-cross-rounded-bold-duotone', 'sign_in.identity_status'],
        ].map(([icon, label]) => (
          <Stack key={label} direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
            <Iconify icon={icon} width={20} sx={{ color: 'primary.main' }} />
            <Typography variant="body2" sx={{ color: 'text.secondary' }}>
              {t(label)}
            </Typography>
          </Stack>
        ))}
      </Stack>

      <Typography variant="body2" sx={{ color: 'text.secondary', textAlign: 'center' }}>
        {t('sign_in.no_account')}{' '}
        <Link
          component={RouterLink}
          href={paths.auth.signUp}
          color="text.primary"
          underline="always"
        >
          {t('sign_in.create_account')}
        </Link>
      </Typography>
    </Stack>
  );
}
