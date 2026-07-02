'use client';

import type { ReactNode, MouseEvent, ChangeEvent } from 'react';

import { z as zod } from 'zod';
import { useMemo, useState } from 'react';
import { useForm } from 'react-hook-form';
import { useBoolean } from 'minimal-shared/hooks';
import { zodResolver } from '@hookform/resolvers/zod';

import Box from '@mui/material/Box';
import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Switch from '@mui/material/Switch';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import ButtonBase from '@mui/material/ButtonBase';
import Typography from '@mui/material/Typography';
import ToggleButton from '@mui/material/ToggleButton';
import { useColorScheme } from '@mui/material/styles';
import InputAdornment from '@mui/material/InputAdornment';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { paths } from 'src/routes/paths';
import { useSearchParams } from 'src/routes/hooks';

import { shouldFail, handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetWalletLnAddress } from 'src/actions/user-wallet';
import { BtcAddressType, changePassword } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error';
import { Form, Field } from 'src/components/hook-form';
import { useSettingsContext } from 'src/components/settings';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

type SettingsSection = 'preferences' | 'receive' | 'security';

const MIN_PASSWORD_LENGTH = 12;

type ChangePasswordFormValues = {
  current_password: string;
  new_password: string;
  confirm_password: string;
};

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
  const showPassword = useBoolean();
  const requestedTab = searchParams.get('tab');
  const [section, setSection] = useState<SettingsSection>(
    requestedTab === 'lnaddress' ? 'receive' : 'preferences'
  );

  const { lnAddress, lnAddressLoading, lnAddressError } = useGetWalletLnAddress();

  const errors = [lnAddressError];
  const isLoading = [lnAddressLoading];
  const failed = shouldFail(errors, [], isLoading);
  const supportsLocalPasswordSettings = CONFIG.auth.method === 'jwt';
  const currentMode = settings.state.mode ?? mode ?? 'system';
  const currentDisplayUnit = settings.state.displayUnit ?? 'bip177';
  const currentHideBalances = settings.state.hideBalances ?? false;
  const currentAddressType = settings.state.defaultAddressType ?? BtcAddressType.P2TR;

  const changePasswordSchema = useMemo(
    () =>
      zod
        .object({
          current_password: zod
            .string()
            .min(1, { message: t('settings_view.current_password_required') }),
          new_password: zod.string().min(MIN_PASSWORD_LENGTH, {
            message: t('settings_view.min_password', { count: MIN_PASSWORD_LENGTH }),
          }),
          confirm_password: zod
            .string()
            .min(1, { message: t('settings_view.confirm_password_required') }),
        })
        .refine((data) => data.new_password === data.confirm_password, {
          path: ['confirm_password'],
          message: t('settings_view.passwords_do_not_match'),
        }),
    [t]
  );

  const passwordMethods = useForm<ChangePasswordFormValues>({
    resolver: zodResolver(changePasswordSchema),
    defaultValues: {
      current_password: '',
      new_password: '',
      confirm_password: '',
    },
  });

  const {
    reset: resetPasswordForm,
    handleSubmit: handlePasswordSubmit,
    formState: { isDirty: passwordFormDirty, isSubmitting: passwordFormSubmitting },
  } = passwordMethods;

  const onPasswordSubmit = handlePasswordSubmit(async (body) => {
    try {
      await changePassword<true>({
        body: {
          current_password: body.current_password,
          new_password: body.new_password,
        },
      });
      toast.success(t('settings_view.password_change_success'));
      resetPasswordForm();
    } catch (error) {
      handleActionError(error);
    }
  });

  const passwordFieldSlotProps = {
    inputLabel: { shrink: true },
    input: {
      endAdornment: (
        <InputAdornment position="end">
          <IconButton
            onClick={showPassword.onToggle}
            edge="end"
            aria-label={t(
              showPassword.value ? 'settings_view.hide_password' : 'settings_view.show_password'
            )}
          >
            <Iconify icon={showPassword.value ? 'solar:eye-bold' : 'solar:eye-closed-bold'} />
          </IconButton>
        </InputAdornment>
      ),
    },
  };

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
                  {supportsLocalPasswordSettings && (
                    <SettingsNavButton
                      active={section === 'security'}
                      icon="solar:lock-password-bold-duotone"
                      title={t('settings_view.security_section')}
                      onClick={() => setSection('security')}
                    />
                  )}
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

              {section === 'security' && supportsLocalPasswordSettings && (
                <SettingsPanel
                  title={t('settings_view.security_section')}
                  description={t('settings_view.security_description')}
                >
                  <Form methods={passwordMethods} onSubmit={onPasswordSubmit}>
                    <Stack spacing={3}>
                      <Alert severity="info" variant="outlined">
                        {t('settings_view.password_change_notice')}
                      </Alert>

                      <Grid container spacing={2}>
                        <Grid size={{ xs: 12 }}>
                          <Field.Text
                            name="current_password"
                            type={showPassword.value ? 'text' : 'password'}
                            label={t('settings_view.current_password')}
                            slotProps={passwordFieldSlotProps}
                          />
                        </Grid>
                        <Grid size={{ xs: 12, sm: 6 }}>
                          <Field.Text
                            name="new_password"
                            type={showPassword.value ? 'text' : 'password'}
                            label={t('settings_view.new_password')}
                            slotProps={passwordFieldSlotProps}
                          />
                        </Grid>
                        <Grid size={{ xs: 12, sm: 6 }}>
                          <Field.Text
                            name="confirm_password"
                            type={showPassword.value ? 'text' : 'password'}
                            label={t('settings_view.confirm_password')}
                            slotProps={passwordFieldSlotProps}
                          />
                        </Grid>
                      </Grid>

                      <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                        <Button
                          type="submit"
                          variant="contained"
                          color="inherit"
                          loading={passwordFormSubmitting}
                          disabled={!passwordFormDirty || passwordFormSubmitting}
                        >
                          {t('settings_view.change_password')}
                        </Button>
                      </Stack>
                    </Stack>
                  </Form>
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
  children?: ReactNode;
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

      {children && <Box sx={{ flexShrink: 0 }}>{children}</Box>}
    </Stack>
  );
}
