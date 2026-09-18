import type { WebhookSubscription } from 'src/lib/swissknife';

import { useState } from 'react';

import { Alert, Stack, Button, TextField, Typography } from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import {
  testWebhook,
  changeWebhook,
  removeWebhook,
  rotateWebhook,
  retryDelivery,
} from 'src/actions/webhooks';

import { toast } from 'src/components/snackbar';
import { ConfirmDialog } from 'src/components/custom-dialog';

import { WebhookForm, SecretDialog } from './webhook-form';

type Props = {
  isAdmin: boolean;
  subscription?: WebhookSubscription;
  id?: string;
  onDeleted: VoidFunction;
  onQueued?: (id: string) => void;
};

export function WebhookActions({ isAdmin, subscription, id, onDeleted, onQueued }: Props) {
  const { t } = useTranslate();
  const [action, setAction] = useState<
    'delete' | 'rotate' | 'disable' | 'enable' | 'test' | 'retry'
  >();
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [secret, setSecret] = useState<string>();
  const [deliveryId, setDeliveryId] = useState('');
  const target = { id: subscription?.id ?? id!, wallet_id: subscription?.wallet_id ?? '' };
  const perform = async () => {
    setBusy(true);
    try {
      if (action === 'delete') {
        await removeWebhook(isAdmin, target);
        onDeleted();
      }
      if (action === 'rotate') setSecret(await rotateWebhook(isAdmin, target));
      if (action === 'disable' || action === 'enable')
        await changeWebhook(isAdmin, target, { active: action === 'enable' });
      if (action === 'test' || action === 'retry') {
        const delivery =
          action === 'test'
            ? await testWebhook(isAdmin, target)
            : await retryDelivery(isAdmin, target, deliveryId);
        toast.success(t('webhook.queued'));
        onQueued?.(delivery.id);
      } else if (action !== 'rotate') toast.success(t('webhook.updated'));
      setAction(undefined);
    } catch (error) {
      handleActionError(error);
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
        <Button variant="outlined" onClick={() => setEditing(true)}>
          {t('webhook.edit')}
        </Button>
        <Button
          variant="outlined"
          onClick={() => setAction('test')}
          disabled={subscription?.active === false}
        >
          {t('webhook.send_test')}
        </Button>
        <Button onClick={() => setAction(subscription?.active === false ? 'enable' : 'disable')}>
          {t(subscription?.active === false ? 'webhook.enable' : 'webhook.disable')}
        </Button>
        {!subscription && (
          <Button onClick={() => setAction('enable')}>{t('webhook.enable')}</Button>
        )}
        <Button onClick={() => setAction('rotate')}>{t('webhook.rotate')}</Button>
        <Button color="error" onClick={() => setAction('delete')}>
          {t('delete')}
        </Button>
      </Stack>
      {!subscription && (
        <Stack spacing={1.5}>
          <Typography variant="subtitle2">{t('webhook.retry_known')}</Typography>
          <TextField
            label={t('webhook.delivery_id')}
            value={deliveryId}
            onChange={(event) => setDeliveryId(event.target.value)}
          />
          <Button
            disabled={!/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(deliveryId)}
            onClick={() => setAction('retry')}
          >
            {t('webhook.retry')}
          </Button>
        </Stack>
      )}
      {editing && (
        <WebhookForm
          isAdmin={isAdmin}
          subscription={subscription}
          targetId={subscription ? undefined : id}
          onClose={() => setEditing(false)}
        />
      )}
      {secret && <SecretDialog secret={secret} onClose={() => setSecret(undefined)} />}
      <ConfirmDialog
        open={!!action}
        onClose={() => !busy && setAction(undefined)}
        title={t(`webhook.confirm_${action ?? 'delete'}_title`)}
        content={
          <Alert severity={action === 'delete' ? 'warning' : 'info'}>
            {t(`webhook.confirm_${action ?? 'delete'}`)}
          </Alert>
        }
        action={
          <Button
            variant="contained"
            color={action === 'delete' ? 'error' : 'primary'}
            loading={busy}
            onClick={perform}
          >
            {t('confirm')}
          </Button>
        }
      />
    </>
  );
}
