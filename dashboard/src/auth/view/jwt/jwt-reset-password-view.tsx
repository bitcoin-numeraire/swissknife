'use client';

import { useState } from 'react';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';

import { paths } from 'src/routes/paths';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { resetLocalPassword } from 'src/lib/swissknife';

import { clearSession } from 'src/auth/context/jwt';

import { FormHead } from '../../components/form-head';

export function JwtResetPasswordView() {
  const { t } = useTranslate();
  const [code, setCode] = useState('');
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [pending, setPending] = useState(false);
  const [complete, setComplete] = useState(false);

  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (password !== confirmation) return;
    setPending(true);
    try {
      await resetLocalPassword<true>({ body: { code: code.trim(), new_password: password } });
      setCode('');
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
            required
            label={t('local_login.new_password')}
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            helperText={t('local_login.password_help')}
          />
          <TextField
            required
            label={t('local_login.confirm_password')}
            type="password"
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
