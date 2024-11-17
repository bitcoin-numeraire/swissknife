'use client';

import type { TFunction } from 'i18next';

import { mutate } from 'swr';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';

import { useBoolean } from 'src/hooks/use-boolean';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { Permission } from 'src/lib/swissknife';
import { useListApiKeys } from 'src/actions/api-key';
import { DashboardContent } from 'src/layouts/dashboard';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { CreateApiKeyDialog } from 'src/components/api-key';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { ApiKeyList } from '../api-key-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'user_id', label: t('api_key_list.user') },
  { id: 'name', label: t('api_key_list.name') },
  { id: 'description', label: t('api_key_list.description') },
  { id: 'permissions', label: t('api_key_list.scopes') },
  { id: 'created_at', label: t('api_key_list.created') },
  { id: 'expires_at', label: t('api_key_list.expires') },
  { id: '' },
];

// ----------------------------------------------------------------------

export function ApiKeyListView() {
  const newApiKey = useBoolean();
  const { t } = useTranslate();

  const { apiKeys, apiKeysLoading, apiKeysError } = useListApiKeys();

  const errors = [apiKeysError];
  const data = [apiKeys];
  const isLoading = [apiKeysLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission.READ_API_KEY]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('api_keys')}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('api_keys'),
                },
              ]}
              action={
                <Stack direction="row" spacing={1}>
                  <Button onClick={newApiKey.onTrue} variant="contained" startIcon={<Iconify icon="mingcute:add-line" />}>
                    {t('new')}
                  </Button>
                </Stack>
              }
              sx={{
                mb: { xs: 3, md: 5 },
              }}
            />

            <ApiKeyList data={apiKeys!} tableHead={tableHead(t)} />

            <CreateApiKeyDialog
              title={t('settings_api_key.new_dialog_title')}
              open={newApiKey.value}
              onClose={newApiKey.onFalse}
              onSuccess={() => {
                mutate(endpointKeys.apiKeys.list);
              }}
              isAdmin
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
