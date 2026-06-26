'use client';

import type { SignInRequest } from 'src/lib/swissknife';

import { useForm } from 'react-hook-form';
import { varAlpha } from 'minimal-shared/utils';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
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
      <Field.Text
        name="password"
        label={t('sign_in.password')}
        type={showPassword.value ? 'text' : 'password'}
        slotProps={{
          inputLabel: { shrink: true },
          input: {
            sx: {
              borderRadius: 1,
              bgcolor: 'background.paper',
              backgroundImage: 'none',
            },
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
        loadingIndicator={t('sign_in.sign_in_loading')}
        sx={{
          py: 1.35,
          borderRadius: 1,
          fontWeight: 700,
          boxShadow: 'none',
        }}
      >
        {t('sign_in.sign_in')}
      </Button>
    </Box>
  );

  return (
    <Stack spacing={3.5}>
      <Box
        sx={(theme) => ({
          width: 150,
          height: 150,
          mx: 'auto',
          display: 'grid',
          position: 'relative',
          placeItems: 'center',
          '&::before': {
            inset: 10,
            content: "''",
            borderRadius: '50%',
            position: 'absolute',
            border: `1px solid ${varAlpha(theme.vars.palette.primary.mainChannel, 0.26)}`,
            backgroundImage: `linear-gradient(180deg, ${varAlpha(theme.vars.palette.primary.mainChannel, 0.14)}, transparent)`,
          },
          '&::after': {
            inset: 0,
            content: "''",
            borderRadius: '50%',
            position: 'absolute',
            border: `1px solid ${varAlpha(theme.vars.palette.grey['500Channel'], 0.16)}`,
          },
        })}
      >
        <AnimateLogoRotate sx={{ zIndex: 1 }} />
        <Box
          sx={(theme) => ({
            right: 20,
            bottom: 18,
            width: 34,
            height: 34,
            zIndex: 2,
            borderRadius: 1,
            display: 'grid',
            position: 'absolute',
            placeItems: 'center',
            color: 'primary.main',
            bgcolor: 'background.paper',
            boxShadow: `0 12px 30px ${varAlpha(theme.vars.palette.common.blackChannel, 0.2)}`,
            border: `1px solid ${varAlpha(theme.vars.palette.primary.mainChannel, 0.24)}`,
          })}
        >
          <Iconify icon="solar:lock-keyhole-minimalistic-bold-duotone" width={20} />
        </Box>
      </Box>

      <FormHead title={t('sign_in.title')} sx={{ mb: 0 }} />

      <Form methods={methods} onSubmit={onSubmit}>
        {renderForm()}
      </Form>
    </Stack>
  );
}
