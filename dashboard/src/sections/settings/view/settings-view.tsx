'use client';

import type { ReactNode, MouseEvent, ChangeEvent } from 'react';

import { useState } from 'react';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Switch from '@mui/material/Switch';
import Divider from '@mui/material/Divider';
import TextField from '@mui/material/TextField';
import ButtonBase from '@mui/material/ButtonBase';
import Typography from '@mui/material/Typography';
import ToggleButton from '@mui/material/ToggleButton';
import { useColorScheme } from '@mui/material/styles';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { paths } from 'src/routes/paths';
import { useSearchParams } from 'src/routes/hooks';

import { shouldFail } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { BtcAddressType } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletApiKeys, useGetWalletLnAddress } from 'src/actions/user-wallet';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error';
import { useSettingsContext } from 'src/components/settings';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

type SettingsSection = 'preferences' | 'receive' | 'security' | 'connections';

const addressTypeOptions = [
  {
    value: BtcAddressType.P2TR,
    labelKey: 'bitcoin_address_type.taproot',
    helperKey: 'bitcoin_address_type.taproot_helper',
  },
  {
    value: BtcAddressType.P2WPKH,
    labelKey: 'bitcoin_address_type.native_segwit',
    helperKey: 'bitcoin_address_type.native_segwit_helper',
  },
] as const;

// ----------------------------------------------------------------------

export function SettingsView() {
  const { t } = useTranslate();
  const settings = useSettingsContext();
  const { mode, setMode } = useColorScheme();
  const searchParams = useSearchParams();
  const requestedTab = searchParams.get('tab');
  const [section, setSection] = useState<SettingsSection>(
    requestedTab === 'lnaddress' ? 'receive' : 'preferences'
  );

  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();
  const { apiKeys, apiKeysLoading, apiKeysError } = useListWalletApiKeys();

  const errors = [lnAddressError, apiKeysError];
  const isLoading = [lnAddressLoading, apiKeysLoading];
  const failed = shouldFail(errors, [apiKeys], isLoading);
  const authModeLabel =
    CONFIG.auth.method === 'jwt'
      ? t('settings_view.jwt_admin_mode')
      : t('settings_view.external_auth_mode');
  const currentMode = settings.state.mode ?? mode ?? 'system';
  const currentDisplayUnit = settings.state.displayUnit ?? 'bip177';
  const currentHideBalances = settings.state.hideBalances ?? false;
  const currentAddressType = settings.state.defaultAddressType ?? BtcAddressType.P2TR;

  const handleModeChange = (
    _: MouseEvent<HTMLElement>,
    value: 'light' | 'dark' | 'system' | null
  ) => {
    if (!value) return;

    setMode(value);
    settings.setState({ mode: value });
  };

  const handleDisplayUnitChange = (_: MouseEvent<HTMLElement>, value: 'bip177' | 'sats' | null) => {
    if (!value) return;

    settings.setState({ displayUnit: value });
  };

  const handleHideBalancesChange = (event: ChangeEvent<HTMLInputElement>) => {
    settings.setState({ hideBalances: event.target.checked });
  };

  const handleAddressTypeChange = (_: MouseEvent<HTMLElement>, value: 'p2tr' | 'p2wpkh' | null) => {
    if (!value) return;

    settings.setState({ defaultAddressType: value });
  };

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('settings')}
            links={[{ name: t('settings_view.manage_account') }]}
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          {requestedTab === 'lnaddress' && (
            <Alert severity="info" variant="outlined" sx={{ mb: 3 }}>
              {t('settings_view.identity_moved')}
            </Alert>
          )}

          <Grid container spacing={3}>
            <Grid size={{ xs: 12, md: 3 }}>
              <Card sx={{ p: 1, borderRadius: 1 }}>
                <Stack spacing={0.5}>
                  <SettingsNavButton
                    active={section === 'preferences'}
                    icon="solar:slider-minimalistic-horizontal-bold-duotone"
                    title={t('settings_view.preferences_section')}
                    onClick={() => setSection('preferences')}
                  />
                  <SettingsNavButton
                    active={section === 'receive'}
                    icon="solar:qr-code-bold-duotone"
                    title={t('settings_view.receive_section')}
                    onClick={() => setSection('receive')}
                  />
                  <SettingsNavButton
                    active={section === 'security'}
                    icon="solar:lock-password-bold-duotone"
                    title={t('settings_view.security_section')}
                    onClick={() => setSection('security')}
                  />
                  <SettingsNavButton
                    active={section === 'connections'}
                    icon="solar:server-bold-duotone"
                    title={t('settings_view.connections_section')}
                    onClick={() => setSection('connections')}
                  />
                </Stack>
              </Card>
            </Grid>

            <Grid size={{ xs: 12, md: 9 }}>
              {section === 'preferences' && (
                <SettingsPanel
                  title={t('settings_view.preferences_section')}
                  description={t('settings_view.preferences_description')}
                >
                  <SettingRow
                    title={t('settings_view.display_unit')}
                    description={t('settings_view.display_unit_description')}
                    status={t('settings_view.local_until_backend')}
                  >
                    <ToggleButtonGroup
                      exclusive
                      size="small"
                      value={currentDisplayUnit}
                      onChange={handleDisplayUnitChange}
                    >
                      <ToggleButton value="bip177">{t('settings_view.unit_bip177')}</ToggleButton>
                      <ToggleButton value="sats">{t('settings_view.unit_sats')}</ToggleButton>
                    </ToggleButtonGroup>
                  </SettingRow>

                  <Divider />

                  <SettingRow
                    title={t('settings_view.hide_balances')}
                    description={t('settings_view.hide_balances_description')}
                    status={t('settings_view.local_until_backend')}
                  >
                    <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                      <Iconify
                        width={22}
                        icon={currentHideBalances ? 'solar:eye-closed-bold' : 'solar:eye-bold'}
                        sx={{ color: currentHideBalances ? 'text.secondary' : 'success.main' }}
                      />
                      <Switch
                        checked={currentHideBalances}
                        onChange={handleHideBalancesChange}
                        slotProps={{ input: { 'aria-label': t('settings_view.hide_balances') } }}
                      />
                    </Stack>
                  </SettingRow>

                  <Divider />

                  <SettingRow
                    title={t('settings_view.theme')}
                    description={t('settings_view.theme_description')}
                  >
                    <ToggleButtonGroup
                      exclusive
                      size="small"
                      value={currentMode}
                      onChange={handleModeChange}
                    >
                      <ToggleButton value="light">{t('settings_view.light')}</ToggleButton>
                      <ToggleButton value="dark">{t('settings_view.dark')}</ToggleButton>
                      <ToggleButton value="system">{t('settings_view.system')}</ToggleButton>
                    </ToggleButtonGroup>
                  </SettingRow>
                </SettingsPanel>
              )}

              {section === 'receive' && (
                <SettingsPanel
                  title={t('settings_view.receive_section')}
                  description={t('settings_view.receive_description')}
                >
                  <SettingRow
                    title={t('settings_view.default_address_type')}
                    description={t('settings_view.default_address_type_description')}
                    status={t('settings_view.local_until_backend')}
                  >
                    <ToggleButtonGroup
                      exclusive
                      size="small"
                      value={currentAddressType}
                      onChange={handleAddressTypeChange}
                    >
                      {addressTypeOptions.map((option) => (
                        <ToggleButton key={option.value} value={option.value}>
                          {t(option.labelKey)}
                        </ToggleButton>
                      ))}
                    </ToggleButtonGroup>
                  </SettingRow>

                  <Divider />

                  <SettingRow
                    title={t('settings_view.lightning_address')}
                    description={t('settings_view.lightning_address_description')}
                    status={
                      lnAddress
                        ? t('settings_view.address_ready')
                        : t('settings_view.address_missing')
                    }
                  >
                    <Button
                      href={`${paths.identity}?tab=lightning`}
                      color="inherit"
                      variant="outlined"
                    >
                      {t('settings_view.open_identity')}
                    </Button>
                  </SettingRow>
                </SettingsPanel>
              )}

              {section === 'security' && (
                <SettingsPanel
                  title={t('settings_view.security_section')}
                  description={t('settings_view.security_description')}
                >
                  <Alert severity="info" variant="outlined">
                    {CONFIG.auth.method === 'jwt'
                      ? t('settings_view.password_backend_needed')
                      : t('settings_view.external_password_notice')}
                  </Alert>

                  <Grid container spacing={2}>
                    <Grid size={{ xs: 12 }}>
                      <TextField
                        disabled
                        fullWidth
                        type="password"
                        label={t('settings_view.current_password')}
                      />
                    </Grid>
                    <Grid size={{ xs: 12, sm: 6 }}>
                      <TextField
                        disabled
                        fullWidth
                        type="password"
                        label={t('settings_view.new_password')}
                      />
                    </Grid>
                    <Grid size={{ xs: 12, sm: 6 }}>
                      <TextField
                        disabled
                        fullWidth
                        type="password"
                        label={t('settings_view.confirm_password')}
                      />
                    </Grid>
                  </Grid>

                  <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                    <Button disabled variant="contained" color="inherit">
                      {t('settings_view.change_password')}
                    </Button>
                    <Label color="warning">{t('settings_view.backend_needed')}</Label>
                  </Stack>
                </SettingsPanel>
              )}

              {section === 'connections' && (
                <SettingsPanel
                  title={t('settings_view.connections_section')}
                  description={t('settings_view.connections_description')}
                >
                  <SettingRow
                    title={t('settings_view.api_keys_tab')}
                    description={t('settings_view.api_keys_description')}
                    status={
                      apiKeys?.length
                        ? t('settings_view.tokens_count', { count: apiKeys.length })
                        : t('settings_view.no_tokens')
                    }
                  >
                    <Button href={paths.build.apiKeys} color="inherit" variant="outlined">
                      {t('settings_view.open_api_keys')}
                    </Button>
                  </SettingRow>

                  <Divider />

                  <SettingRow
                    title={t('settings_view.instance_access')}
                    description={
                      CONFIG.auth.method === 'jwt'
                        ? t('settings_view.jwt_instance_description')
                        : t('settings_view.external_instance_description')
                    }
                    status={`${CONFIG.deploymentMode} · ${authModeLabel}`}
                  >
                    <Button href={paths.nodeHealth} color="inherit" variant="outlined">
                      {t('settings_view.open_node_health')}
                    </Button>
                  </SettingRow>
                </SettingsPanel>
              )}
            </Grid>
          </Grid>
        </>
      )}
    </DashboardContent>
  );
}

function SettingsNavButton({
  active,
  icon,
  title,
  onClick,
}: {
  active: boolean;
  icon: string;
  title: string;
  onClick: VoidFunction;
}) {
  return (
    <ButtonBase
      onClick={onClick}
      sx={[
        (theme) => ({
          p: 1.5,
          gap: 1.25,
          width: 1,
          display: 'flex',
          borderRadius: 1,
          textAlign: 'left',
          alignItems: 'center',
          justifyContent: 'flex-start',
          color: active ? 'text.primary' : 'text.secondary',
          bgcolor: active ? 'background.neutral' : 'transparent',
          border: `1px solid ${active ? theme.vars.palette.divider : 'transparent'}`,
        }),
      ]}
    >
      <Iconify icon={icon} width={22} />
      <Typography variant="subtitle2">{title}</Typography>
    </ButtonBase>
  );
}

function SettingsPanel({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <Card sx={{ p: 3, borderRadius: 1 }}>
      <Stack spacing={3}>
        <Stack spacing={0.5}>
          <Typography variant="h5">{title}</Typography>
          <Typography variant="body2" color="text.secondary">
            {description}
          </Typography>
        </Stack>
        {children}
      </Stack>
    </Card>
  );
}

function SettingRow({
  title,
  description,
  status,
  children,
}: {
  title: string;
  description: string;
  status?: string;
  children: ReactNode;
}) {
  return (
    <Stack
      direction={{ xs: 'column', md: 'row' }}
      spacing={2}
      sx={{ alignItems: { md: 'center' }, justifyContent: 'space-between' }}
    >
      <Stack spacing={0.75} sx={{ maxWidth: 520 }}>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <Typography variant="subtitle1">{title}</Typography>
          {status && <Label color="info">{status}</Label>}
        </Stack>
        <Typography variant="body2" color="text.secondary">
          {description}
        </Typography>
      </Stack>

      <Box sx={{ flexShrink: 0 }}>{children}</Box>
    </Stack>
  );
}
