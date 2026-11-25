'use client';

import type { IdentityAction } from './wallet-identity-dialog';
import type { IdentityChange, SealedIdentity } from 'src/lib/wallet-identity/vault';

import useSWR from 'swr';
import { useState } from 'react';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { npub } from 'src/utils/nostr';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';
import { getWalletIdentity, setWalletIdentity } from 'src/lib/swissknife';
import {
  sameIdentity,
  stageIdentity,
  deviceIdentity,
  finishIdentity,
} from 'src/lib/wallet-identity/vault';

import { requestId } from 'src/sections/wallet/request-id';

import { IdentityDialog } from './wallet-identity-dialog';

export function WalletIdentityPanel({ walletId }: { walletId: string }) {
  const { account } = useAccountContext();
  const { t } = useTranslate();
  const key = account?.id ? JSON.stringify([CONFIG.serverUrl, account.id, walletId]) : '';
  const { data, error, mutate } = useSWR(key ? ['wallet-identity', key] : null, async () => {
    const [{ data: remote }, local] = await Promise.all([
      getWalletIdentity({ path: { wallet_id: walletId }, throwOnError: true }),
      deviceIdentity(key),
    ]);
    return { remote: remote ?? null, local };
  });
  const [action, setAction] = useState<IdentityAction>();
  const [working, setWorking] = useState(false);
  const [message, setMessage] = useState('');
  const sync = async (pending: IdentityChange, discard = false) => {
    const { data: current } = await getWalletIdentity({
      path: { wallet_id: walletId },
      throwOnError: true,
    });
    if (sameIdentity(current, pending.sealed.identity)) {
      await finishIdentity(key, pending.id, true);
    } else if (discard) {
      if (sameIdentity(current, pending.expected)) throw new Error('retry_pending');
      await finishIdentity(key, pending.id, false);
    } else {
      await setWalletIdentity({
        path: { wallet_id: walletId },
        body: { identity: pending.sealed.identity, expected: pending.expected },
        throwOnError: true,
      });
      await finishIdentity(key, pending.id, true);
    }
  };
  const resume = async (discard: boolean) => {
    if (!data?.local.pending) return;
    setWorking(true);
    setMessage('');
    try {
      await sync(data.local.pending, discard);
    } catch {
      setMessage(t('wallet_keys.sync_error'));
    } finally {
      await mutate();
      setWorking(false);
    }
  };
  const save = async (sealed: SealedIdentity) => {
    const change = { id: requestId(), expected: data?.remote ?? null, sealed };
    await stageIdentity(key, change);
    setAction(undefined);
    setMessage('');
    try {
      await sync(change);
    } catch {
      setMessage(t('wallet_keys.sync_error'));
    } finally {
      await mutate();
    }
  };
  if (error) return <Alert severity="error">{t('wallet_keys.load_error')}</Alert>;
  if (!data) return <Typography>{t('wallet_keys.loading')}</Typography>;
  const available = Boolean(
    data.local.active && sameIdentity(data.local.active.identity, data.remote)
  );
  return (
    <Stack spacing={2} sx={{ alignItems: 'flex-start' }}>
      <Typography variant="h6">{t('wallet_keys.title')}</Typography>
      <Typography variant="body2" color="text.secondary">
        {t('wallet_keys.purpose')}
      </Typography>
      {data.remote && (
        <Stack spacing={0.5}>
          <Typography variant="caption">{t('wallet_keys.nostr_public')}</Typography>
          <Typography variant="body2" sx={{ overflowWrap: 'anywhere' }}>
            {npub(data.remote.nostr_pubkey)}
          </Typography>
        </Stack>
      )}
      {message && <Alert severity="warning">{message}</Alert>}
      {data.local.pending ? (
        <>
          <Alert severity="info">{t('wallet_keys.pending')}</Alert>
          <Stack direction="row" spacing={1}>
            <Button
              variant="contained"
              color="inherit"
              disabled={working}
              onClick={() => resume(false)}
            >
              {t('wallet_keys.retry')}
            </Button>
            {!sameIdentity(data.remote, data.local.pending.expected) && (
              <Button disabled={working} onClick={() => resume(true)}>
                {t('wallet_keys.discard')}
              </Button>
            )}
          </Stack>
        </>
      ) : (
        <Stack direction="row" sx={{ flexWrap: 'wrap', gap: 1 }}>
          {!available && !data.remote && (
            <Button variant="contained" color="inherit" onClick={() => setAction('create')}>
              {t('wallet_keys.create')}
            </Button>
          )}
          {!available && (
            <Button variant="outlined" color="inherit" onClick={() => setAction('restore')}>
              {t('wallet_keys.restore')}
            </Button>
          )}
          {available &&
            (['login', 'backup', 'rotate', 'restore'] as const).map((item) => (
              <Button
                key={item}
                variant={item === 'login' ? 'contained' : 'outlined'}
                color="inherit"
                onClick={() => setAction(item)}
              >
                {t(`wallet_keys.${item}`)}
              </Button>
            ))}
        </Stack>
      )}
      {action && (
        <IdentityDialog
          key={action}
          action={action}
          scope={key}
          active={data.local.active}
          remote={data.remote}
          onSave={save}
          onClose={() => setAction(undefined)}
        />
      )}
    </Stack>
  );
}
