'use client';

import { useState, useEffect } from 'react';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';
import { updateCurrentAccount } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

export function DisplayNameForm() {
  const { t } = useTranslate();
  const { account, refreshAccount } = useAccountContext();
  const [displayName, setDisplayName] = useState(account?.display_name ?? '');
  const [pending, setPending] = useState(false);

  useEffect(() => {
    setDisplayName(account?.display_name ?? '');
  }, [account?.display_name]);

  const save = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPending(true);
    try {
      await updateCurrentAccount<true>({ body: { display_name: displayName.trim() || null } });
      await refreshAccount();
      toast.success(t('settings_view.display_name_saved'));
    } catch (error) {
      handleActionError(error);
    } finally {
      setPending(false);
    }
  };

  return (
    <Stack component="form" onSubmit={save} spacing={2}>
      <TextField
        label={t('accounts_view.display_name')}
        helperText={t('settings_view.display_name_help')}
        autoComplete="nickname"
        value={displayName}
        onChange={(event) => setDisplayName(event.target.value)}
      />
      <Button
        type="submit"
        variant="contained"
        loading={pending}
        disabled={!account || displayName.trim() === (account.display_name ?? '')}
        sx={{ alignSelf: 'flex-start' }}
      >
        {t('save')}
      </Button>
    </Stack>
  );
}
