import type { Wallet, WebhookSubscription } from 'src/lib/swissknife';

import { z as zod } from 'zod';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import {
  Alert,
  Stack,
  Button,
  Dialog,
  Checkbox,
  MenuItem,
  TextField,
  DialogTitle,
  DialogActions,
  DialogContent,
  InputAdornment,
  FormControlLabel,
} from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useListWallets } from 'src/actions/wallet';
import { useAccountContext } from 'src/contexts/account';
import { Permission, ClientEventType } from 'src/lib/swissknife';
import { saveWebhook, changeWebhook } from 'src/actions/webhooks';
import { zCreateWebhookSubscriptionRequest } from 'src/lib/swissknife/zod.gen';

import { toast } from 'src/components/snackbar';
import { CopyButton } from 'src/components/copy';
import { Form, Field } from 'src/components/hook-form';

import { useAuthContext } from 'src/auth/hooks';

export function SecretDialog({ secret, onClose }: { secret: string; onClose: VoidFunction }) {
  const { t } = useTranslate();
  const [saved, setSaved] = useState(false);
  return (
    <Dialog open onClose={saved ? onClose : undefined} fullWidth maxWidth="sm">
      <DialogTitle>{t('webhook.secret_title')}</DialogTitle>
      <DialogContent>
        <Stack spacing={3} sx={{ pt: 1 }}>
          <Alert severity="warning">{t('webhook.secret_once')}</Alert>
          <TextField
            label={t('webhook.signing_secret')}
            value={secret}
            slotProps={{
              input: {
                readOnly: true,
                endAdornment: (
                  <InputAdornment position="end">
                    <CopyButton value={secret} />
                  </InputAdornment>
                ),
              },
            }}
          />
          <FormControlLabel
            control={<Checkbox checked={saved} onChange={(_, checked) => setSaved(checked)} />}
            label={t('webhook.saved_secret')}
          />
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button variant="contained" disabled={!saved} onClick={onClose}>
          {t('done')}
        </Button>
      </DialogActions>
    </Dialog>
  );
}

export function walletLabel(wallet: Wallet) {
  return [wallet.label || wallet.asset?.name, wallet.asset?.network, wallet.id]
    .filter(Boolean)
    .join(' · ');
}

function WalletOptions({ wallets, showOwner = false }: { wallets: Wallet[]; showOwner?: boolean }) {
  const { t } = useTranslate();
  return (
    <Field.Select name="wallet_id" label={t('webhook.wallet')}>
      {wallets.map((wallet) => (
        <MenuItem
          key={wallet.id}
          value={wallet.id}
          sx={{ whiteSpace: 'normal', overflowWrap: 'anywhere' }}
        >
          {walletLabel(wallet)}
          {showOwner && ` · ${t('webhook.account_id')}: ${wallet.account_id}`}
        </MenuItem>
      ))}
    </Field.Select>
  );
}

function AdminWalletOptions() {
  const { t } = useTranslate();
  const { wallets, walletsLoading, walletsError } = useListWallets();
  if (walletsError)
    return (
      <>
        <Alert severity="warning">{t('webhook.wallet_load_error')}</Alert>
        <Field.Text name="wallet_id" label={t('webhook.wallet_id')} />
      </>
    );
  if (walletsLoading) return <Alert severity="info">{t('loading')}</Alert>;
  return <WalletOptions wallets={wallets ?? []} showOwner />;
}

type Props = {
  isAdmin: boolean;
  subscription?: WebhookSubscription;
  targetId?: string;
  onClose: VoidFunction;
  onCreated?: (id: string, walletId: string) => void;
};

export function WebhookForm({ isAdmin, subscription, targetId, onClose, onCreated }: Props) {
  const { t } = useTranslate();
  const { user } = useAuthContext();
  const { wallets, activeWalletId } = useAccountContext();
  const [secret, setSecret] = useState<string>();
  const [created, setCreated] = useState<{ id: string; walletId: string }>();
  const writeOnly = !!targetId && !subscription;
  const editing = !!subscription || !!targetId;
  const schema = zCreateWebhookSubscriptionRequest.extend({
    wallet_id: editing ? zod.string().optional() : zod.uuid(t('webhook.wallet_required')),
    url: zod.string().refine((value) => {
      if (writeOnly && !value) return true;
      try {
        const url = new URL(value);
        return url.protocol === 'https:' && !url.username && !url.password && !url.hash;
      } catch {
        return false;
      }
    }, t('webhook.https_required')),
    event_types: writeOnly
      ? zod.array(zod.enum(ClientEventType))
      : zod.array(zod.enum(ClientEventType)).min(1, t('webhook.events_required')),
  });
  const methods = useForm({
    resolver: zodResolver(schema),
    defaultValues: {
      wallet_id: subscription?.wallet_id ?? (isAdmin ? '' : (activeWalletId ?? '')),
      url: subscription?.url ?? '',
      event_types: subscription?.event_types ?? [],
    },
  });
  const submit = methods.handleSubmit(async (values) => {
    try {
      if (writeOnly) {
        await changeWebhook(
          true,
          { id: targetId!, wallet_id: '' },
          {
            url: values.url || undefined,
            event_types: values.event_types.length ? values.event_types : undefined,
          }
        );
        toast.success(t('webhook.updated'));
        onClose();
        return;
      }
      const data = await saveWebhook(
        isAdmin,
        values.wallet_id ?? subscription?.wallet_id ?? '',
        values,
        subscription?.id
      );
      if ('signing_secret' in data && typeof data.signing_secret === 'string') {
        setCreated({ id: data.id, walletId: data.wallet_id });
        setSecret(data.signing_secret);
      } else {
        toast.success(t('webhook.updated'));
        onClose();
      }
    } catch (error) {
      handleActionError(error);
    }
  });
  if (secret)
    return (
      <SecretDialog
        secret={secret}
        onClose={() => {
          setSecret(undefined);
          onClose();
          if (created) onCreated?.(created.id, created.walletId);
        }}
      />
    );
  return (
    <Dialog
      open
      fullWidth
      maxWidth="sm"
      onClose={methods.formState.isSubmitting ? undefined : onClose}
    >
      <DialogTitle>{t(editing ? 'webhook.edit' : 'webhook.create')}</DialogTitle>
      <Form methods={methods} onSubmit={submit}>
        <DialogContent>
          <Stack spacing={3} sx={{ pt: 1 }}>
            {writeOnly && <Alert severity="info">{t('webhook.write_only_edit')}</Alert>}
            {!editing &&
              (isAdmin ? (
                user?.permissions.includes(Permission.READ_WALLET) ? (
                  <AdminWalletOptions />
                ) : (
                  <Field.Text
                    name="wallet_id"
                    label={t('webhook.wallet_id')}
                    helperText={t('webhook.wallet_id_help')}
                  />
                )
              ) : (
                <WalletOptions wallets={wallets} />
              ))}
            {subscription && (
              <Alert severity="info">
                {t('webhook.wallet')}: {subscription.wallet_id}
              </Alert>
            )}
            <Field.Text
              name="url"
              label={t('webhook.destination')}
              placeholder="https://example.com/webhooks"
              helperText={t('webhook.destination_help')}
            />
            <Field.MultiCheckbox
              name="event_types"
              label={t('webhook.events')}
              options={Object.values(ClientEventType).map((value) => ({ value, label: value }))}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose} disabled={methods.formState.isSubmitting}>
            {t('cancel')}
          </Button>
          <Button
            type="submit"
            variant="contained"
            loading={methods.formState.isSubmitting}
            disabled={writeOnly && !methods.watch('url') && !methods.watch('event_types').length}
          >
            {t(editing ? 'save' : 'webhook.create')}
          </Button>
        </DialogActions>
      </Form>
    </Dialog>
  );
}
