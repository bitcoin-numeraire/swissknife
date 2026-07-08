'use client';

import type { TFunction } from 'i18next';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { Permission } from 'src/lib/swissknife';
import { useListApiKeys } from 'src/actions/api-key';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletApiKeys } from 'src/actions/user-wallet';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { CreateApiKeyDrawer } from 'src/components/api-key';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { ApiKeyList } from '../api-key-list';
import { SettingsApiKey } from '../../settings/settings-api-key';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'account_id', label: t('api_key_list.account') },
  { id: 'name', label: t('api_key_list.name') },
  { id: 'description', label: t('api_key_list.description') },
  { id: 'permissions', label: t('api_key_list.scopes') },
  { id: 'created_at', label: t('api_key_list.created') },
  { id: 'expires_at', label: t('api_key_list.expires') },
  { id: '' },
];

// ----------------------------------------------------------------------

export function ApiKeyListView() {
  const { t } = useTranslate();

  const { apiKeys, apiKeysLoading, apiKeysError } = useListWalletApiKeys();

  const errors = [apiKeysError];
  const data = [apiKeys];
  const isLoading = [apiKeysLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('api_keys')}
            links={[
              {
                name: t('build'),
              },
              {
                name: t('api_keys'),
              },
            ]}
            sx={{
              mb: { xs: 3, md: 5 },
            }}
          />

          <Stack spacing={3}>
            <SettingsApiKey apiKeys={apiKeys!} />

            <RoleBasedGuard permissions={[Permission.READ_API_KEY]}>
              <InstanceApiKeysPanel />
            </RoleBasedGuard>
          </Stack>
        </>
      )}
    </DashboardContent>
  );
}

function InstanceApiKeysPanel() {
  const newApiKey = useBoolean();
  const { t } = useTranslate();

  const { apiKeys, apiKeysLoading, apiKeysError } = useListApiKeys();

  const errors = [apiKeysError];
  const data = [apiKeys];
  const isLoading = [apiKeysLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <Stack spacing={2.5}>
          <Stack
            direction={{ xs: 'column', sm: 'row' }}
            spacing={2}
            sx={{ alignItems: { sm: 'center' }, justifyContent: 'space-between' }}
          >
            <Stack spacing={0.5}>
              <Typography variant="h5">{t('api_key_list.instance_title')}</Typography>
              <Typography variant="body2" color="text.secondary">
                {t('api_key_list.instance_description')}
              </Typography>
            </Stack>

            <Button
              onClick={newApiKey.onTrue}
              variant="contained"
              startIcon={<Iconify icon="mingcute:add-line" />}
            >
              {t('new')}
            </Button>
          </Stack>

          <ApiKeyList data={apiKeys!} tableHead={tableHead(t)} />

          <CreateApiKeyDrawer
            title={t('settings_api_key.new_dialog_title')}
            open={newApiKey.value}
            onClose={newApiKey.onFalse}
            onSuccess={() => {
              mutate(endpointKeys.apiKeys.list);
            }}
            isAdmin
          />
        </Stack>
      )}
    </>
  );
}
