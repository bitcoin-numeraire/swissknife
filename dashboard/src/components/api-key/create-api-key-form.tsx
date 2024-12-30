import type { Permission, ApiKeyResponse, CreateApiKeyRequest } from 'src/lib/swissknife';

import { useState } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { useForm, FormProvider } from 'react-hook-form';

import { LoadingButton } from '@mui/lab';
import {
  Link,
  Stack,
  Alert,
  Divider,
  MenuItem,
  TextField,
  Typography,
  InputAdornment,
} from '@mui/material';

import { fDate } from 'src/utils/format-time';
import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { CONFIG } from 'src/global-config';
import { zCreateApiKeyRequest } from 'src/lib/swissknife/zod.gen';
import { createApiKey, createWalletApiKey } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { RHFSelect, RHFTextField, RHFMultiCheckbox } from 'src/components/hook-form';

import { useAuthContext } from 'src/auth/hooks';

import { CopyButton } from '../copy';
import { WalletSelectDropdown } from '../wallet';

// ----------------------------------------------------------------------

const expiryOptions = [
  { label: '30 days', value: 30 * 24 * 60 * 60 },
  { label: '60 days', value: 60 * 24 * 60 * 60 },
  { label: '90 days', value: 90 * 24 * 60 * 60 },
  { label: '1 year', value: 365 * 24 * 60 * 60 },
];

type Props = {
  onSuccess: VoidFunction;
  isAdmin?: boolean;
};

const permissionOptions = (permissions: Permission[]) =>
  permissions.map((value) => ({ label: value, value }));

export function CreateApiKeyForm({ onSuccess, isAdmin }: Props) {
  const { t } = useTranslate();
  const { user } = useAuthContext();
  const [apiKey, setApiKey] = useState<ApiKeyResponse>();

  const methods = useForm({
    resolver: zodResolver(zCreateApiKeyRequest),
    defaultValues: {
      name: '',
      user_id: user?.sub,
      expiry: expiryOptions[2].value,
      permissions: [],
    },
  });

  const {
    reset,
    handleSubmit,
    formState: { isSubmitting, isValid },
  } = methods;

  const onSubmit = async (body: CreateApiKeyRequest) => {
    try {
      if (isAdmin) {
        const { data } = await createApiKey({ body });
        setApiKey(data);
      } else {
        const { data } = await createWalletApiKey({ body });
        setApiKey(data);
      }
      toast.success(t('create_api_key_form.create_success'));
      reset();
      onSuccess();
    } catch (error) {
      handleActionError(error);
    }
  };

  return apiKey && apiKey.key ? (
    <Stack spacing={2}>
      <Alert severity="warning">{t('create_api_key_form.key_display_message')}</Alert>
      <TextField
        value={apiKey.key}
        InputProps={{
          readOnly: true,
          endAdornment: (
            <InputAdornment position="end">
              <CopyButton value={apiKey.key} title={t('create_api_key_form.copy')} />
            </InputAdornment>
          ),
        }}
      />
    </Stack>
  ) : (
    <FormProvider {...methods}>
      <form onSubmit={handleSubmit(onSubmit)}>
        <Stack spacing={3}>
          <RHFTextField
            autoFocus
            variant="outlined"
            name="name"
            label={t('create_api_key_form.name')}
          />
          <RHFTextField
            variant="outlined"
            fullWidth
            name="description"
            label={t('create_api_key_form.description')}
          />
          <RHFSelect name="expiry" label={t('create_api_key_form.expiration')}>
            <MenuItem value={0}>
              <Typography variant="body1">Never expires</Typography>
            </MenuItem>
            <Divider sx={{ borderStyle: 'dashed' }} />

            {expiryOptions.map((option) => (
              <MenuItem key={option.value} value={option.value}>
                <div>
                  <Typography variant="body1">{option.label}</Typography>
                  <Typography variant="caption" color="text.secondary">
                    Expires {fDate(Date.now() + option.value * 1000, 'DD MMMM YYYY')}
                  </Typography>
                </div>
              </MenuItem>
            ))}
          </RHFSelect>

          {user!.permissions.length > 0 ? (
            <RHFMultiCheckbox
              row
              name="permissions"
              label="Scopes"
              options={permissionOptions(user!.permissions)}
              sx={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, 1fr)',
                gap: 1,
                mx: 'auto',
              }}
            />
          ) : (
            <Alert variant="filled" severity="info">
              {t('create_api_key_form.no_permissions')}:{' '}
              <Link href={`${CONFIG.serverUrl}/docs#tag/user-wallet`} target="_blank">
                See Docs
              </Link>
            </Alert>
          )}

          {isAdmin && <WalletSelectDropdown name="user_id" />}

          <LoadingButton
            type="submit"
            variant="contained"
            color="inherit"
            size="large"
            loading={isSubmitting}
            disabled={!isValid}
          >
            {t('create_api_key_form.create_button')}
          </LoadingButton>
        </Stack>
      </form>
    </FormProvider>
  );
}
