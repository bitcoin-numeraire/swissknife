'use client';

import type { Account, LocalLoginReset } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useState } from 'react';

import Card from '@mui/material/Card';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';

import { paths } from 'src/routes/paths';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { endpointKeys } from 'src/actions/keys';
import { useAccountContext } from 'src/contexts/account';
import {
  useLocalLogin,
  addLocalLogin,
  setLocalLoginEnabled,
  issueLocalLoginReset,
} from 'src/actions/local-login';

import { Label } from 'src/components/label';
import { CopyButton } from 'src/components/copy';
import { ConfirmDialog } from 'src/components/custom-dialog';

import { RoleBasedGuard } from 'src/auth/guard';

export function LocalLoginCard({ account }: { account: Account }) {
  const { t } = useTranslate();
  const { account: currentAccount } = useAccountContext();
  const { data, error, isLoading, mutate: refresh } = useLocalLogin(account.id);
  const login = data?.data;
  const [username, setUsername] = useState(account.identity?.subject ?? '');
  const [grant, setGrant] = useState<LocalLoginReset | null>(null);
  const [pending, setPending] = useState(false);
  const [confirmation, setConfirmation] = useState<'reset' | 'disable' | null>(null);
  const isSelf = currentAccount?.id === account.id;

  const run = async (action: 'create' | 'reset' | 'disable' | 'enable') => {
    setPending(true);
    try {
      if (action === 'create') {
        const result = await addLocalLogin(account.id, username);
        setGrant(result.data);
      } else if (action === 'reset') {
        const result = await issueLocalLoginReset(account.id);
        setGrant(result.data);
      } else {
        await setLocalLoginEnabled(account.id, action === 'enable');
        setGrant(null);
      }
      setConfirmation(null);
      await Promise.all([
        refresh(),
        mutate(endpointKeys.accounts.get(account.id)),
        mutate(endpointKeys.accounts.list),
      ]);
    } catch (cause) {
      handleActionError(cause);
    } finally {
      setPending(false);
    }
  };

  return (
    <Card sx={{ p: 3, borderRadius: 1 }}>
      <Stack spacing={2}>
        <Typography variant="h6">{t('local_login.title')}</Typography>
        {error ? (
          <Alert severity="error">{t('local_login.load_error')}</Alert>
        ) : isLoading ? (
          <Typography>{t('local_login.loading')}</Typography>
        ) : (
          <>
            {login ? (
              <Stack spacing={1}>
                <Typography>
                  {t('local_login.username')}: {login.username}
                </Typography>
                <Label
                  color={login.enabled ? 'success' : 'warning'}
                  sx={{ alignSelf: 'flex-start' }}
                >
                  {t(
                    !login.enabled
                      ? 'local_login.disabled'
                      : login.password_set
                        ? 'local_login.enabled'
                        : 'local_login.awaiting_password'
                  )}
                </Label>
                {login.reset_expires_at && (
                  <Typography variant="body2">
                    {t('local_login.code_expires', {
                      date: new Date(login.reset_expires_at).toLocaleString(),
                    })}
                  </Typography>
                )}
              </Stack>
            ) : (
              <Typography variant="body2">{t('local_login.no_login')}</Typography>
            )}
            <Typography variant="body2" color="text.secondary">
              {t('local_login.api_keys_separate')}
            </Typography>
            <RoleBasedGuard permissions={[Permission.WRITE_ACCOUNT]}>
              {!login ? (
                <Stack spacing={2}>
                  <TextField
                    label={t('local_login.username')}
                    helperText={t('local_login.username_help')}
                    disabled={!!account.identity}
                    value={username}
                    onChange={(event) => setUsername(event.target.value)}
                    autoComplete="off"
                    slotProps={{ htmlInput: { maxLength: 64 } }}
                  />
                  <Button
                    variant="contained"
                    loading={pending}
                    disabled={!account.identity && !/^[a-z0-9._-]{3,64}$/i.test(username.trim())}
                    onClick={() => run('create')}
                    sx={{ alignSelf: 'flex-start' }}
                  >
                    {t('local_login.create')}
                  </Button>
                </Stack>
              ) : isSelf ? (
                <Button href={paths.settings.root} sx={{ alignSelf: 'flex-start' }}>
                  {t('local_login.change_own_password')}
                </Button>
              ) : (
                <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1}>
                  {login.enabled ? (
                    <>
                      <Button
                        variant="outlined"
                        disabled={pending}
                        onClick={() => setConfirmation('reset')}
                      >
                        {t('local_login.reset')}
                      </Button>
                      <Button
                        color="warning"
                        variant="outlined"
                        disabled={pending}
                        onClick={() => setConfirmation('disable')}
                      >
                        {t('local_login.disable')}
                      </Button>
                    </>
                  ) : (
                    <Button variant="outlined" loading={pending} onClick={() => run('enable')}>
                      {t('local_login.enable')}
                    </Button>
                  )}
                </Stack>
              )}
            </RoleBasedGuard>
          </>
        )}
        {grant && (
          <Alert severity="info" onClose={() => setGrant(null)}>
            <Stack spacing={1}>
              <Typography variant="subtitle2">{t('local_login.code_title')}</Typography>
              <Typography variant="body2">{t('local_login.code_help')}</Typography>
              <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                <Typography sx={{ fontFamily: 'monospace', overflowWrap: 'anywhere' }}>
                  {grant.code}
                </Typography>
                <CopyButton value={grant.code} />
              </Stack>
              <Typography variant="body2">
                {t('local_login.code_expires', {
                  date: new Date(grant.expires_at).toLocaleString(),
                })}
              </Typography>
            </Stack>
          </Alert>
        )}
      </Stack>
      <ConfirmDialog
        open={confirmation !== null}
        onClose={() => setConfirmation(null)}
        title={t(confirmation === 'disable' ? 'local_login.disable' : 'local_login.reset')}
        content={t(
          confirmation === 'disable'
            ? 'local_login.disable_confirmation'
            : 'local_login.reset_confirmation'
        )}
        action={
          <Button
            variant="contained"
            loading={pending}
            onClick={() => confirmation && run(confirmation)}
          >
            {t('confirm')}
          </Button>
        }
      />
    </Card>
  );
}
