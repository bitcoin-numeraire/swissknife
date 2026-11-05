'use client';

import type { TextFieldProps } from '@mui/material/TextField';

import { useState } from 'react';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import IconButton from '@mui/material/IconButton';
import InputAdornment from '@mui/material/InputAdornment';

import { paths } from 'src/routes/paths';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { resetLocalPassword } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';

import { clearSession } from 'src/auth/context/jwt';

import { FormHead } from '../../components/form-head';

export function JwtResetPasswordView() {
  const { t } = useTranslate();
  const [code, setCode] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [pending, setPending] = useState(false);
  const [complete, setComplete] = useState(false);

  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (password !== confirmation) return;
    setPending(true);
    try {
      await resetLocalPassword<true>({
        body: {
          code: code.trim(),
          new_password: password,
          display_name: displayName.trim() || null,
        },
      });
      setCode('');
      setDisplayName('');
      setPassword('');
      setConfirmation('');
      clearSession();
      setComplete(true);
    } catch (error) {
      handleActionError(error);
    } finally {
      setPending(false);
    }
  };

  return (
    <Stack spacing={3}>
      <FormHead title={t('local_login.set_password')} description={t('local_login.reset_help')} />
      {complete ? (
        <Alert severity="success">{t('local_login.reset_success')}</Alert>
      ) : (
        <Stack component="form" onSubmit={submit} spacing={3}>
          <TextField
            required
            label={t('local_login.code')}
            value={code}
            onChange={(event) => setCode(event.target.value)}
            autoComplete="off"
            slotProps={{ htmlInput: { maxLength: 43 } }}
          />
          <TextField
            label={t('local_login.display_name')}
            helperText={t('local_login.reset_display_name_help')}
            autoComplete="nickname"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
          <PasswordField
            required
            label={t('local_login.new_password')}
            autoComplete="new-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            helperText={t('local_login.password_help')}
          />
          <PasswordField
            required
            label={t('local_login.confirm_password')}
            autoComplete="new-password"
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
            error={!!confirmation && password !== confirmation}
            helperText={
              confirmation && password !== confirmation
                ? t('sign_up.passwords_do_not_match')
                : undefined
            }
          />
          <Button
            type="submit"
            variant="contained"
            loading={pending}
            disabled={
              Array.from(password).length < 15 ||
              new TextEncoder().encode(password).length > 1024 ||
              password !== confirmation ||
              code.trim().length !== 43
            }
          >
            {t('local_login.set_password')}
          </Button>
        </Stack>
      )}
      <Button href={paths.auth.login}>{t('local_login.back_to_login')}</Button>
    </Stack>
  );
}

function PasswordField(props: Omit<TextFieldProps, 'type' | 'slotProps'>) {
  const { t } = useTranslate();
  const [visible, setVisible] = useState(false);

  return (
    <TextField
      {...props}
      type={visible ? 'text' : 'password'}
      slotProps={{
        input: {
          endAdornment: (
            <InputAdornment position="end">
              <IconButton
                type="button"
                edge="end"
                aria-label={t(
                  visible ? 'settings_view.hide_password' : 'settings_view.show_password'
                )}
                aria-pressed={visible}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => setVisible((current) => !current)}
              >
                <Iconify icon={visible ? 'solar:eye-bold' : 'solar:eye-closed-bold'} />
              </IconButton>
            </InputAdornment>
          ),
        },
      }}
    />
  );
}
