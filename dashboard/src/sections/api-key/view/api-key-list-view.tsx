'use client';

import { mutate } from 'swr';
import { useState } from 'react';

import { Alert, Stack, Button, TextField, Typography } from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { revokeApiKey } from 'src/lib/swissknife';
import { useListApiKeys } from 'src/actions/api-key';
import { useListAccountApiKeys } from 'src/actions/account-wallet';

import { toast } from 'src/components/snackbar';
import { ErrorView } from 'src/components/error';
import { CreateApiKeyDrawer } from 'src/components/api-key';
import { ConfirmDialog } from 'src/components/custom-dialog';

import { ApiKeyList } from '../api-key-list';
import { SettingsApiKey } from '../../settings/settings-api-key';

type Props = { isAdmin: boolean; canRead: boolean; canWrite: boolean };

export function ApiKeysPanel({ isAdmin, canRead, canWrite }: Props) {
  return isAdmin ? (
    <InstanceApiKeysPanel canRead={canRead} canWrite={canWrite} />
  ) : (
    <MyApiKeysPanel />
  );
}

function MyApiKeysPanel() {
  const { apiKeys, apiKeysLoading, apiKeysError } = useListAccountApiKeys();
  return !apiKeys ? (
    <ErrorView errors={[apiKeysError]} isLoading={[apiKeysLoading]} data={[apiKeys]} />
  ) : (
    <SettingsApiKey apiKeys={apiKeys} />
  );
}

function InstanceApiKeysPanel({ canRead, canWrite }: Omit<Props, 'isAdmin'>) {
  const { t } = useTranslate();
  const [creating, setCreating] = useState(false);
  const [keyId, setKeyId] = useState('');
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const revoke = async () => {
    setBusy(true);
    try {
      await revokeApiKey<true>({ path: { id: keyId } });
      setKeyId('');
      setConfirm(false);
      toast.success(t('settings_api_key.revoke_success'));
      await mutate(endpointKeys.apiKeys.list);
    } catch (error) {
      handleActionError(error);
    } finally {
      setBusy(false);
    }
  };
  return (
    <Stack spacing={3}>
      <Stack direction="row" sx={{ justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="h5">{t('api_key_list.instance_title')}</Typography>
        {canWrite && (
          <Button variant="contained" onClick={() => setCreating(true)}>
            {t('new')}
          </Button>
        )}
      </Stack>
      {canRead ? (
        <InstanceApiKeysList canWrite={canWrite} />
      ) : (
        <>
          <Alert severity="info">{t('developers.api_keys_write_only')}</Alert>
          <TextField
            label={t('developers.api_key_id')}
            value={keyId}
            onChange={(event) => setKeyId(event.target.value)}
          />
          <Button
            color="error"
            disabled={!/^[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i.test(keyId)}
            onClick={() => setConfirm(true)}
          >
            {t('revoke')}
          </Button>
        </>
      )}
      {creating && (
        <CreateApiKeyDrawer
          title={t('settings_api_key.new_dialog_title')}
          open
          onClose={() => setCreating(false)}
          onSuccess={() => {
            mutate(endpointKeys.apiKeys.list);
          }}
          isAdmin
        />
      )}
      <ConfirmDialog
        open={confirm}
        onClose={() => !busy && setConfirm(false)}
        title={t('confirm_delete_title')}
        content={keyId}
        action={
          <Button color="error" variant="contained" loading={busy} onClick={revoke}>
            {t('revoke')}
          </Button>
        }
      />
    </Stack>
  );
}

function InstanceApiKeysList({ canWrite }: { canWrite: boolean }) {
  const { t } = useTranslate();
  const { apiKeys, apiKeysLoading, apiKeysError } = useListApiKeys();
  if (!apiKeys)
    return <ErrorView errors={[apiKeysError]} isLoading={[apiKeysLoading]} data={[apiKeys]} />;
  return (
    <ApiKeyList
      data={apiKeys}
      canWrite={canWrite}
      tableHead={[
        { id: 'account_id', label: t('api_key_list.account') },
        { id: 'name', label: t('api_key_list.name') },
        { id: 'description', label: t('api_key_list.description') },
        { id: 'permissions', label: t('api_key_list.scopes') },
        { id: 'created_at', label: t('api_key_list.created') },
        { id: 'expires_at', label: t('api_key_list.expires') },
        { id: '' },
      ]}
    />
  );
}
