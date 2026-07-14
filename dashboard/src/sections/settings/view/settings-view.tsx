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
import Avatar from '@mui/material/Avatar';
import Button from '@mui/material/Button';
import Switch from '@mui/material/Switch';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import ButtonBase from '@mui/material/ButtonBase';
import Typography from '@mui/material/Typography';
import ToggleButton from '@mui/material/ToggleButton';
import InputAdornment from '@mui/material/InputAdornment';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { paths } from 'src/routes/paths';
import { useSearchParams } from 'src/routes/hooks';

import { shouldFail, handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useAccountContext } from 'src/contexts/account';
import { useGetAccountLnAddress } from 'src/actions/account-wallet';
import { BtcAddressType, changePassword } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { ErrorView } from 'src/components/error';
import { Form, Field } from 'src/components/hook-form';
import { useSettingsContext } from 'src/components/settings';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { useAuthContext } from 'src/auth/hooks';

// ----------------------------------------------------------------------

type SettingsSection = 'profile' | 'preferences' | 'receive' | 'security';

type SessionUserProfile = {
  sub?: string;
  name?: string;
  email?: string;
  picture?: string;
  photoURL?: string;
  displayName?: string;
  email_verified?: boolean;
  permissions?: unknown[];
  identities?: Array<{ provider?: string }>;
  app_metadata?: { provider?: string };
};

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

const identityProviderLabels: Record<string, string> = {
  apple: 'Apple',
  auth0: 'Auth0',
  facebook: 'Facebook',
  github: 'GitHub',
  google: 'Google',
  'google-oauth2': 'Google',
  email: 'Email',
  jwt: 'JWT',
  linkedin: 'LinkedIn',
  oauth2: 'OAuth2',
  twitter: 'X',
  'twitter-v2': 'X',
  windowslive: 'Microsoft',
};

function compactValue(value: string) {
  return value.length > 28 ? `${value.slice(0, 12)}...${value.slice(-8)}` : value;
}

function identityProviderLabel(user: SessionUserProfile | null, provider?: string) {
  const explicitProvider = user?.identities?.[0]?.provider ?? user?.app_metadata?.provider;
  const subjectProvider = user?.sub?.includes('|') ? user.sub.split('|')[0] : undefined;
  const providerKey = String(explicitProvider ?? subjectProvider ?? provider ?? '').toLowerCase();

  if (identityProviderLabels[providerKey]) return identityProviderLabels[providerKey];
  if (provider === 'oauth2' && CONFIG.auth.method === 'auth0') return 'Auth0';
  return providerKey || 'Unknown';
}

// ----------------------------------------------------------------------

export function SettingsView() {
  const { t } = useTranslate();
  const { user } = useAuthContext();
  const sessionUser = user as SessionUserProfile | null;
  const settings = useSettingsContext();
  const { account, accountLoading, accountError, updateDashboardPreferences } = useAccountContext();
  const searchParams = useSearchParams();
  const showPassword = useBoolean();
  const requestedTab = searchParams.get('tab');
  const [section, setSection] = useState<SettingsSection>(
    requestedTab === 'lnaddress' ? 'receive' : 'profile'
  );

  const { lnAddress, lnAddressLoading, lnAddressError } = useGetAccountLnAddress();

  const errors = [accountError, lnAddressError];
  const isLoading = [accountLoading, lnAddressLoading];
  const failed = shouldFail(errors, [account], isLoading);
  const supportsLocalPasswordSettings = CONFIG.auth.method === 'jwt';
  const currentMode = settings.state.mode ?? 'system';
  const currentDisplayUnit = settings.state.displayUnit ?? 'bip177';
  const currentHideBalances = settings.state.hideBalances ?? false;
  const currentAddressType = settings.state.defaultAddressType ?? BtcAddressType.P2TR;
  const profileName =
    sessionUser?.displayName ||
    sessionUser?.name ||
    account?.display_name ||
    account?.identity?.subject ||
    t('settings_view.profile_fallback');
  const profileEmail = sessionUser?.email;
  const profilePhoto = sessionUser?.photoURL || sessionUser?.picture;
  const identitySubject = account?.identity?.subject || sessionUser?.sub;
  const providerLabel = identityProviderLabel(sessionUser, account?.identity?.provider);
  const effectivePermissions = Array.isArray(sessionUser?.permissions)
    ? sessionUser.permissions.filter((permission: unknown) => typeof permission === 'string')
    : [];

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

    updateDashboardPreferences({ mode: value }).catch(handleActionError);
  };

  const handleDisplayUnitChange = (_: MouseEvent<HTMLElement>, value: 'bip177' | 'sats' | null) => {
    if (!value) return;

    updateDashboardPreferences({ displayUnit: value }).catch(handleActionError);
  };

  const handleHideBalancesChange = (event: ChangeEvent<HTMLInputElement>) => {
    updateDashboardPreferences({ hideBalances: event.target.checked }).catch(handleActionError);
  };

  const handleAddressTypeChange = (_: MouseEvent<HTMLElement>, value: 'p2tr' | 'p2wpkh' | null) => {
    if (!value) return;

    updateDashboardPreferences({ defaultAddressType: value }).catch(handleActionError);
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
                    active={section === 'profile'}
                    icon="solar:user-id-bold-duotone"
                    title={t('settings_view.profile_section')}
                    onClick={() => setSection('profile')}
                  />
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
              {section === 'profile' && (
                <SettingsPanel
                  title={t('settings_view.profile_section')}
                  description={t('settings_view.profile_description')}
                >
                  <Stack
                    direction={{ xs: 'column', sm: 'row' }}
                    spacing={2}
                    sx={{ alignItems: { sm: 'center' } }}
                  >
                    <Avatar
                      src={profilePhoto}
                      alt={profileName}
                      sx={{ width: 72, height: 72, typography: 'h4' }}
                    >
                      {profileName.charAt(0).toUpperCase()}
                    </Avatar>
                    <Stack spacing={0.5} sx={{ minWidth: 0 }}>
                      <Typography variant="h5" noWrap>
                        {profileName}
                      </Typography>
                      {profileEmail && (
                        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                          <Typography variant="body2" color="text.secondary" noWrap>
                            {profileEmail}
                          </Typography>
                          {typeof sessionUser?.email_verified === 'boolean' && (
                            <Label color={sessionUser.email_verified ? 'success' : 'warning'}>
                              {t(
                                sessionUser.email_verified
                                  ? 'settings_view.email_verified'
                                  : 'settings_view.email_unverified'
                              )}
                            </Label>
                          )}
                        </Stack>
                      )}
                    </Stack>
                  </Stack>

                  <Divider />

                  <SettingRow
                    title={t('accounts_view.account_id')}
                    description={t('settings_view.account_id_description')}
                  >
                    {account?.id && <Identifier value={account.id} />}
                  </SettingRow>

                  <Divider />

                  <SettingRow
                    title={t('settings_view.sign_in_provider')}
                    description={t('settings_view.sign_in_provider_description')}
                  >
                    <Label color="info">{providerLabel}</Label>
                  </SettingRow>

                  {identitySubject && (
                    <>
                      <Divider />
                      <SettingRow
                        title={t('settings_view.identity_subject')}
                        description={t('settings_view.identity_subject_description')}
                      >
                        <Identifier value={identitySubject} />
                      </SettingRow>
                    </>
                  )}

                  {effectivePermissions.length > 0 && (
                    <>
                      <Divider />
                      <Stack spacing={1.5}>
                        <Stack spacing={0.5}>
                          <Typography variant="subtitle1">
                            {t('settings_view.effective_permissions')}
                          </Typography>
                          <Typography variant="body2" color="text.secondary">
                            {t('settings_view.effective_permissions_description')}
                          </Typography>
                        </Stack>
                        <Stack direction="row" spacing={0.75} useFlexGap sx={{ flexWrap: 'wrap' }}>
                          {effectivePermissions.map((permission: string) => (
                            <Label key={permission}>{permission}</Label>
                          ))}
                        </Stack>
                      </Stack>
                    </>
                  )}
                </SettingsPanel>
              )}

              {section === 'preferences' && (
                <SettingsPanel
                  title={t('settings_view.preferences_section')}
                  description={t('settings_view.preferences_description')}
                >
                  <SettingRow
                    title={t('settings_view.display_unit')}
                    description={t('settings_view.display_unit_description')}
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

function Identifier({ value }: { value: string }) {
  const { t } = useTranslate();

  return (
    <Stack direction="row" spacing={0.25} sx={{ alignItems: 'center', minWidth: 0 }}>
      <Typography variant="body2" sx={{ fontFamily: 'monospace' }} noWrap>
        {compactValue(value)}
      </Typography>
      <CopyButton value={value} title={t('copy')} />
    </Stack>
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
