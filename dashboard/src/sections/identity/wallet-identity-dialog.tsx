import type { WalletIdentity } from 'src/lib/swissknife';
import type { IdentitySecrets } from 'src/lib/wallet-identity/keys';
import type { SealedIdentity } from 'src/lib/wallet-identity/vault';

import { useState } from 'react';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import Checkbox from '@mui/material/Checkbox';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import DialogTitle from '@mui/material/DialogTitle';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import FormControlLabel from '@mui/material/FormControlLabel';

import { useTranslate } from 'src/locales';
import { sameIdentity, sealIdentity, unlockIdentity } from 'src/lib/wallet-identity/vault';
import {
  nostrKeys,
  signLogin,
  reviewLogin,
  newIdentityPhrase,
  importIdentityPhrase,
} from 'src/lib/wallet-identity/keys';

export type IdentityAction = 'create' | 'restore' | 'backup' | 'rotate' | 'login';
const secretInput = { autoComplete: 'off', spellCheck: false, autoCapitalize: 'none' } as const;

export function IdentityDialog({
  action,
  scope,
  active,
  remote,
  onSave,
  onClose,
}: {
  action: IdentityAction;
  scope: string;
  active?: SealedIdentity;
  remote: WalletIdentity | null;
  onSave: (value: SealedIdentity) => Promise<void>;
  onClose: () => void;
}) {
  const { t } = useTranslate();
  const [phrase, setPhrase] = useState(() => (action === 'create' ? newIdentityPhrase() : ''));
  const [password, setPassword] = useState('');
  const [bip39Passphrase, setBip39Passphrase] = useState('');
  const [override, setOverride] = useState('');
  const [confirmed, setConfirmed] = useState(false);
  const [challenge, setChallenge] = useState('');
  const [approvedUrl, setApprovedUrl] = useState('');
  const [revealed, setRevealed] = useState<{ phrase: string; nsec: string; passphrase: string }>();
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const setup = action === 'create' || action === 'restore';
  let review: ReturnType<typeof reviewLogin> | undefined;
  if (action === 'login' && challenge) {
    try {
      review = reviewLogin(challenge);
    } catch {
      /* Keep incomplete QR input editable. */
    }
  }
  const submit = async () => {
    setBusy(true);
    setError('');
    try {
      let secrets: IdentitySecrets;
      if (setup) {
        secrets = {
          phrase: await importIdentityPhrase(phrase),
          bip39Passphrase,
          nostrSecret: override.trim() || undefined,
        };
      } else {
        if (!active) throw Error();
        try {
          secrets = await unlockIdentity(scope, active, password);
        } catch {
          setError(t('wallet_keys.unlock_error'));
          return;
        }
      }
      if (action === 'login' && review) {
        setApprovedUrl(await signLogin(secrets, review.url));
        setPassword('');
        return;
      }
      if (action === 'rotate') secrets = { ...secrets, nostrSecret: override.trim() || undefined };
      const derived = await nostrKeys(secrets);
      if (action === 'backup') {
        setRevealed({
          phrase: secrets.phrase,
          nsec: derived.nsec,
          passphrase: secrets.bip39Passphrase,
        });
        setPassword('');
        return;
      }
      if (setup && remote && !sameIdentity(remote, derived.identity)) {
        setError(t('wallet_keys.restore_mismatch'));
        return;
      }
      await onSave(await sealIdentity(scope, derived.identity, secrets, password));
    } catch {
      setError(t('wallet_keys.input_error'));
    } finally {
      setBusy(false);
    }
  };
  const ready = Boolean(
    password &&
    (!setup || (password.length >= 12 && phrase && confirmed)) &&
    (action !== 'login' || review)
  );
  return (
    <Dialog open onClose={busy ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t(`wallet_keys.${action}`)}</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ pt: 1 }}>
          {error && <Alert severity="error">{error}</Alert>}
          {approvedUrl && <Typography>{t('wallet_keys.continue_help')}</Typography>}
          {revealed && (
            <>
              <TextField
                multiline
                label={t('wallet_keys.phrase')}
                value={revealed.phrase}
                slotProps={{ input: { readOnly: true } }}
              />
              {revealed.passphrase && (
                <TextField
                  label={t('wallet_keys.bip39_password')}
                  value={revealed.passphrase}
                  slotProps={{ input: { readOnly: true } }}
                />
              )}
              <TextField
                multiline
                label={t('wallet_keys.nsec')}
                value={revealed.nsec}
                slotProps={{ input: { readOnly: true } }}
              />
            </>
          )}
          {!revealed && !approvedUrl && (
            <>
              {setup && (
                <>
                  <Alert severity="info">{t('wallet_keys.backup_help')}</Alert>
                  <TextField
                    multiline
                    minRows={2}
                    label={t('wallet_keys.phrase')}
                    value={phrase}
                    onChange={(e) => setPhrase(e.target.value)}
                    slotProps={{ input: { readOnly: action === 'create' }, htmlInput: secretInput }}
                  />
                  {action === 'restore' && (
                    <TextField
                      type="password"
                      label={t('wallet_keys.bip39_password')}
                      value={bip39Passphrase}
                      onChange={(e) => setBip39Passphrase(e.target.value)}
                      autoComplete="off"
                    />
                  )}
                </>
              )}
              {(action === 'rotate' ||
                (action === 'restore' &&
                  remote &&
                  remote.nostr_pubkey !== remote.recovery_pubkey)) && (
                <TextField
                  type="password"
                  label={t('wallet_keys.nsec')}
                  helperText={t('wallet_keys.override_help')}
                  value={override}
                  onChange={(e) => setOverride(e.target.value)}
                  autoComplete="off"
                />
              )}
              {action === 'login' && (
                <>
                  <TextField
                    label={t('wallet_keys.challenge')}
                    value={challenge}
                    onChange={(e) => setChallenge(e.target.value)}
                    slotProps={{ htmlInput: secretInput }}
                    error={Boolean(challenge && !review)}
                    helperText={challenge && !review ? t('wallet_keys.invalid_login') : undefined}
                  />
                  {review && (
                    <Alert severity="info">
                      {t('wallet_keys.login_review', {
                        action: t(`wallet_keys.actions.${review.action}`),
                        domain: new URL(review.url).host,
                      })}
                    </Alert>
                  )}
                </>
              )}
              <TextField
                label={t('wallet_keys.password')}
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                autoComplete={setup ? 'new-password' : 'off'}
                helperText={setup ? t('wallet_keys.password_help') : undefined}
              />
              {setup && (
                <FormControlLabel
                  control={
                    <Checkbox
                      checked={confirmed}
                      onChange={(e) => setConfirmed(e.target.checked)}
                    />
                  }
                  label={t('wallet_keys.saved')}
                />
              )}
            </>
          )}
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button disabled={busy} onClick={onClose}>
          {t('close')}
        </Button>
        {approvedUrl ? (
          <Button
            component="a"
            href={approvedUrl}
            target="_blank"
            rel="noopener noreferrer"
            referrerPolicy="no-referrer"
            variant="contained"
            color="inherit"
            onClick={onClose}
          >
            {t('wallet_keys.continue')}
          </Button>
        ) : (
          !revealed && (
            <Button
              variant="contained"
              color="inherit"
              loading={busy}
              disabled={!ready}
              onClick={submit}
            >
              {t(
                action === 'login'
                  ? 'wallet_keys.approve'
                  : action === 'backup'
                    ? 'wallet_keys.reveal'
                    : 'wallet_keys.save'
              )}
            </Button>
          )
        )}
      </DialogActions>
    </Dialog>
  );
}
