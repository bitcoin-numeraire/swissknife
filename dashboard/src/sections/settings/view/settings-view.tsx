'use client';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { paths } from 'src/routes/paths';
import { useSearchParams } from 'src/routes/hooks';

import { shouldFail } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletApiKeys, useGetWalletLnAddress } from 'src/actions/user-wallet';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

export function SettingsView() {
  const { t } = useTranslate();
  const searchParams = useSearchParams();
  const requestedTab = searchParams.get('tab');

  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();
  const { apiKeys, apiKeysLoading, apiKeysError } = useListWalletApiKeys();

  const errors = [lnAddressError, apiKeysError];
  const isLoading = [lnAddressLoading, apiKeysLoading];
  // `lnAddress` is legitimately null when no address is registered.
  const data = [apiKeys];

  const failed = shouldFail(errors, data, isLoading);
  const identityHref = `${paths.identity}?tab=lightning`;
  const apiKeysHref = paths.build.apiKeys;
  const authModeLabel =
    CONFIG.auth.method === 'jwt'
      ? t('settings_view.jwt_admin_mode')
      : t('settings_view.external_auth_mode');

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('settings')}
            links={[
              {
                name: t('settings_view.manage_account'),
              },
            ]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <Card sx={{ p: 3, mb: 3, borderRadius: 1 }}>
            <Stack
              direction={{ xs: 'column', md: 'row' }}
              spacing={3}
              sx={{ alignItems: { md: 'center' }, justifyContent: 'space-between' }}
            >
              <Stack spacing={0.75} sx={{ maxWidth: 640 }}>
                <Typography variant="h4">{t('settings_view.control_center')}</Typography>
                <Typography variant="body2" color="text.secondary">
                  {t('settings_view.subtitle')}
                </Typography>
              </Stack>

              <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5}>
                <SettingsStatus
                  icon="solar:bolt-bold-duotone"
                  title={t('settings_view.lightning_tab')}
                  value={
                    lnAddress
                      ? t('settings_view.address_ready')
                      : t('settings_view.address_missing')
                  }
                  color={lnAddress ? 'success.main' : 'warning.main'}
                />
                <SettingsStatus
                  icon="solar:key-minimalistic-square-bold-duotone"
                  title={t('settings_view.api_keys_tab')}
                  value={
                    apiKeys?.length
                      ? t('settings_view.tokens_count', { count: apiKeys.length })
                      : t('settings_view.no_tokens')
                  }
                  color={apiKeys?.length ? 'info.main' : 'text.disabled'}
                />
                <SettingsStatus
                  icon="solar:server-bold-duotone"
                  title={t('settings_view.access_tab')}
                  value={authModeLabel}
                  color={CONFIG.auth.method === 'jwt' ? 'warning.main' : 'success.main'}
                />
              </Stack>
            </Stack>
          </Card>

          {requestedTab === 'lnaddress' && (
            <Alert
              severity="info"
              variant="outlined"
              sx={{ mb: 3 }}
              action={
                <Button color="inherit" size="small" href={identityHref}>
                  {t('settings_view.open_identity')}
                </Button>
              }
            >
              {t('settings_view.identity_moved')}
            </Alert>
          )}

          <Grid container spacing={3}>
            <Grid size={{ xs: 12, md: 4 }}>
              <SettingsRouteCard
                icon="solar:user-rounded-bold-duotone"
                title={t('identity_hub')}
                value={
                  lnAddress ? t('settings_view.address_ready') : t('settings_view.address_missing')
                }
                description={t('settings_view.identity_description')}
                actionLabel={t('settings_view.open_identity')}
                href={identityHref}
                color={lnAddress ? 'success.main' : 'warning.main'}
              />
            </Grid>

            <Grid size={{ xs: 12, md: 4 }}>
              <SettingsRouteCard
                icon="solar:key-minimalistic-square-bold-duotone"
                title={t('api_keys')}
                value={
                  apiKeys?.length
                    ? t('settings_view.tokens_count', { count: apiKeys.length })
                    : t('settings_view.no_tokens')
                }
                description={t('settings_view.api_keys_description')}
                actionLabel={t('settings_view.open_api_keys')}
                href={apiKeysHref}
                color={apiKeys?.length ? 'info.main' : 'text.disabled'}
              />
            </Grid>

            <Grid size={{ xs: 12, md: 4 }}>
              <SettingsRouteCard
                icon="solar:server-bold-duotone"
                title={t('settings_view.instance_access')}
                value={`${CONFIG.deploymentMode} · ${authModeLabel}`}
                description={
                  CONFIG.auth.method === 'jwt'
                    ? t('settings_view.jwt_instance_description')
                    : t('settings_view.external_instance_description')
                }
                actionLabel={t('settings_view.open_node_health')}
                href={paths.nodeHealth}
                color={CONFIG.auth.method === 'jwt' ? 'warning.main' : 'success.main'}
              />
            </Grid>
          </Grid>
        </>
      )}
    </DashboardContent>
  );
}

function SettingsRouteCard({
  icon,
  title,
  value,
  color,
  description,
  actionLabel,
  href,
}: {
  icon: string;
  title: string;
  value: string;
  color: string;
  description: string;
  actionLabel: string;
  href: string;
}) {
  return (
    <Card sx={{ p: 3, borderRadius: 1, height: 1 }}>
      <Stack spacing={2.5} sx={{ height: 1 }}>
        <Box
          sx={{
            width: 48,
            height: 48,
            display: 'grid',
            borderRadius: 1,
            placeItems: 'center',
            color,
            bgcolor: 'background.neutral',
          }}
        >
          <Iconify icon={icon} width={28} />
        </Box>

        <Stack spacing={0.75}>
          <Typography variant="h6">{title}</Typography>
          <Typography variant="body2" color="text.secondary">
            {description}
          </Typography>
        </Stack>

        <Box sx={{ flexGrow: 1 }} />

        <Stack spacing={1.5}>
          <Typography variant="subtitle2" color={color}>
            {value}
          </Typography>
          <Button href={href} color="inherit" variant="outlined">
            {actionLabel}
          </Button>
        </Stack>
      </Stack>
    </Card>
  );
}

function SettingsStatus({
  icon,
  title,
  value,
  color,
}: {
  icon: string;
  title: string;
  value: string;
  color: string;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: 1.5,
          minWidth: { sm: 190 },
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
        }),
      ]}
    >
      <Stack direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
        <Iconify icon={icon} width={26} sx={{ color }} />
        <Stack sx={{ minWidth: 0 }}>
          <Typography variant="caption" color="text.secondary">
            {title}
          </Typography>
          <Typography variant="subtitle2" noWrap>
            {value}
          </Typography>
        </Stack>
      </Stack>
    </Box>
  );
}
