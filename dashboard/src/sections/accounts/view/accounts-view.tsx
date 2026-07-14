'use client';

import type { Wallet, Account, Permission as PermissionValue } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useState, useEffect } from 'react';
import { useBoolean } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Chip from '@mui/material/Chip';
import Grid from '@mui/material/Grid';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import Checkbox from '@mui/material/Checkbox';
import TextField from '@mui/material/TextField';
import FormGroup from '@mui/material/FormGroup';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import FormControlLabel from '@mui/material/FormControlLabel';

import { paths } from 'src/routes/paths';
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { fDate } from 'src/utils/format-time';
import { fSats } from 'src/utils/format-number';
import { displayLnAddress } from 'src/utils/lnurl';
import { shouldFail, handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { useGetAccount } from 'src/actions/account';
import { DashboardContent } from 'src/layouts/dashboard';
import {
  Permission,
  AuthProvider,
  createAccount,
  deleteAccountById,
  updateAccountById,
  replaceAccountPermissions,
} from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { RegisterWalletDrawer } from 'src/components/wallet';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import {
  DetailRow,
  DetailCard,
  MetricTile,
  formatDateTime,
} from 'src/sections/transaction/transaction-detail-common';

import { RoleBasedGuard } from 'src/auth/guard';

import { AccountDirectory } from './account-directory';

// ----------------------------------------------------------------------

const permissionOptions = Object.values(Permission);

export { accountMatchesSearch } from './account-directory';

function accountName(account: Account) {
  return account.display_name || account.identity?.subject || account.id;
}

function compactId(id: string) {
  return id.length > 20 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;
}

function formatDashboardSettings(value: unknown) {
  const formatted = JSON.stringify(value, null, 2);
  return formatted ?? String(value);
}

function WalletAssetIcon({ wallet }: { wallet: Wallet }) {
  const isNativeBitcoin = wallet.asset?.protocol === 'bitcoin';
  const ticker = wallet.asset?.display_ticker || wallet.asset?.code || '?';

  return (
    <Box
      sx={(theme) => ({
        width: 44,
        height: 44,
        display: 'grid',
        flexShrink: 0,
        borderRadius: '50%',
        placeItems: 'center',
        color: theme.vars.palette.warning.main,
        bgcolor: 'background.neutral',
        border: `1px solid ${theme.vars.palette.divider}`,
      })}
    >
      {isNativeBitcoin ? (
        <Box
          component="img"
          alt="Bitcoin"
          src={`${CONFIG.assetsDir}/assets/icons/bitcoin/ic-bitcoin.svg`}
          sx={{ width: 30, height: 30 }}
        />
      ) : (
        <Typography variant="caption" sx={{ fontWeight: 700 }}>
          {ticker.slice(0, 4)}
        </Typography>
      )}
    </Box>
  );
}

function WalletBalance({ wallet }: { wallet: Wallet }) {
  if (!wallet.asset || wallet.asset.protocol === 'bitcoin') {
    return (
      <SatsWithIcon
        amountMSats={wallet.balance.available_msat}
        variant="h4"
        sx={{ fontWeight: 400 }}
      />
    );
  }

  const scale = 10 ** wallet.asset.decimals;
  const amount = wallet.balance.available_msat / scale;

  return (
    <Typography variant="h4" sx={{ fontWeight: 400 }}>
      {fSats(amount, { maximumFractionDigits: Math.min(wallet.asset.decimals, 8) })}{' '}
      <Typography component="span" variant="subtitle2" color="text.secondary">
        {wallet.asset.display_ticker}
      </Typography>
    </Typography>
  );
}

function AccountWalletCard({ wallet }: { wallet: Wallet }) {
  const { t } = useTranslate();
  const lnAddress = wallet.ln_address?.username
    ? displayLnAddress(wallet.ln_address.username)
    : t('wallets_view.missing');

  return (
    <Card sx={{ p: 2.5, borderRadius: 1, height: 1 }}>
      <Stack spacing={2} sx={{ height: 1 }}>
        <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
          <WalletAssetIcon wallet={wallet} />
          <Stack sx={{ minWidth: 0, flex: 1 }}>
            <Typography variant="subtitle1" noWrap>
              {wallet.label || wallet.asset?.name || wallet.asset?.display_ticker || t('wallet')}
            </Typography>
            <Typography variant="caption" color="text.secondary" noWrap>
              {wallet.asset?.display_ticker ?? wallet.asset_id}
            </Typography>
          </Stack>
          <Label color="info">{wallet.asset?.network ?? '-'}</Label>
        </Stack>

        <Stack spacing={0.5}>
          <Typography variant="caption" color="text.secondary">
            {t('accounts_view.spendable_balance')}
          </Typography>
          <WalletBalance wallet={wallet} />
        </Stack>

        <Divider sx={{ borderStyle: 'dashed' }} />

        <Stack spacing={1}>
          <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2 }}>
            <Typography variant="caption" color="text.secondary">
              {t('lightning_address')}
            </Typography>
            <Typography variant="caption" noWrap>
              {lnAddress}
            </Typography>
          </Stack>
          <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2 }}>
            <Typography variant="caption" color="text.secondary">
              {t('wallet_list.created')}
            </Typography>
            <Typography variant="caption">{fDate(wallet.created_at)}</Typography>
          </Stack>
          <Stack direction="row" sx={{ justifyContent: 'space-between', gap: 2 }}>
            <Typography variant="caption" color="text.secondary">
              {t('wallet')}
            </Typography>
            <Typography variant="caption" sx={{ fontFamily: 'monospace' }}>
              {compactId(wallet.id)}
            </Typography>
          </Stack>
        </Stack>

        <Button
          href={paths.admin.wallet(wallet.id)}
          color="inherit"
          variant="soft"
          endIcon={<Iconify icon="eva:arrow-ios-forward-fill" />}
          sx={{ mt: 'auto' }}
        >
          {t('details')}
        </Button>
      </Stack>
    </Card>
  );
}

export function AccountsView() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const accountId = searchParams.get('id');
  const [editorAccount, setEditorAccount] = useState<Account | null>();

  if (accountId) return <AccountDetailsView id={accountId} />;

  return (
    <>
      <AccountDirectory onCreate={() => setEditorAccount(null)} />
      <AccountEditorDrawer
        open={editorAccount !== undefined}
        account={editorAccount ?? null}
        onClose={() => setEditorAccount(undefined)}
        onSuccess={(account) => {
          setEditorAccount(undefined);
          router.push(paths.admin.account(account.id));
        }}
      />
    </>
  );
}

function AccountDetailsView({ id }: { id: string }) {
  const { t } = useTranslate();
  const router = useRouter();
  const editor = useBoolean();
  const newWallet = useBoolean();
  const confirmDelete = useBoolean();
  const isDeleting = useBoolean();
  const { account, accountLoading, accountError } = useGetAccount(id);
  const accountWallets = account?.wallets ?? [];
  const errors = [accountError];
  const data = [account];
  const isLoading = [accountLoading];
  const failed = shouldFail(errors, data, isLoading);

  const handleDelete = async () => {
    isDeleting.onTrue();
    try {
      await deleteAccountById<true>({ path: { id } });
      await mutate(endpointKeys.accounts.list);
      toast.success(t('accounts_view.delete_success'));
      router.push(paths.admin.accounts);
    } catch (error) {
      handleActionError(error);
    } finally {
      isDeleting.onFalse();
      confirmDelete.onFalse();
    }
  };

  return (
    <DashboardContent maxWidth="xl">
      <RoleBasedGuard permissions={[Permission.READ_ACCOUNT]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={accountName(account!)}
              links={[
                { name: t('accounts') },
                { name: t('accounts_directory'), href: paths.admin.accounts },
                { name: t('details') },
              ]}
              action={
                <RoleBasedGuard permissions={[Permission.WRITE_ACCOUNT]}>
                  <Stack direction="row" spacing={1}>
                    <Button
                      color="inherit"
                      variant="outlined"
                      startIcon={<Iconify icon="solar:pen-bold" />}
                      onClick={editor.onTrue}
                    >
                      {t('edit')}
                    </Button>
                    <Button
                      color="error"
                      variant="outlined"
                      startIcon={<Iconify icon="solar:trash-bin-trash-bold" />}
                      onClick={confirmDelete.onTrue}
                    >
                      {t('delete')}
                    </Button>
                  </Stack>
                </RoleBasedGuard>
              }
              sx={{ mb: { xs: 3, md: 5 } }}
            />

            <Stack spacing={3}>
              <Card sx={{ p: 3, borderRadius: 1 }}>
                <Grid container spacing={3} sx={{ alignItems: 'stretch' }}>
                  <Grid size={{ xs: 12, md: 5 }}>
                    <Stack spacing={2} sx={{ height: 1, justifyContent: 'center' }}>
                      <Box
                        sx={(theme) => ({
                          width: 56,
                          height: 56,
                          display: 'grid',
                          borderRadius: 1,
                          placeItems: 'center',
                          color: theme.vars.palette.info.main,
                          bgcolor: 'background.paper',
                          border: `1px solid ${theme.vars.palette.info.main}`,
                        })}
                      >
                        <Iconify icon="solar:user-id-bold-duotone" width={32} />
                      </Box>
                      <Stack spacing={1}>
                        <Stack
                          direction="row"
                          spacing={1}
                          useFlexGap
                          sx={{ alignItems: 'center', flexWrap: 'wrap' }}
                        >
                          <Typography variant="h4">{accountName(account!)}</Typography>
                        </Stack>
                        <Typography
                          variant="body2"
                          color="text.secondary"
                          sx={{ fontFamily: 'monospace', wordBreak: 'break-word' }}
                        >
                          {account!.id}
                        </Typography>
                      </Stack>
                    </Stack>
                  </Grid>

                  <Grid size={{ xs: 12, md: 7 }}>
                    <Grid container spacing={2}>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <MetricTile
                          title={t('wallets')}
                          value={accountWallets.length.toLocaleString()}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <MetricTile
                          title={t('accounts_view.permissions')}
                          value={
                            account!.identity?.provider === AuthProvider.OAUTH2
                              ? t('accounts_view.provider_managed')
                              : (account!.permissions?.length ?? 0).toLocaleString()
                          }
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <MetricTile
                          title={t('wallet_list.created')}
                          value={fDate(account!.created_at)}
                        />
                      </Grid>
                      <Grid size={{ xs: 12, sm: 6 }}>
                        <MetricTile
                          title={t('wallet_list.updated')}
                          value={formatDateTime(account!.updated_at)}
                        />
                      </Grid>
                    </Grid>
                  </Grid>
                </Grid>
              </Card>

              <Grid container spacing={3}>
                <Grid size={{ xs: 12, md: 6 }}>
                  <DetailCard
                    title={t('accounts_view.account_record')}
                    icon="solar:users-group-two-rounded-bold-duotone"
                    color="info"
                  >
                    <DetailRow
                      label={t('accounts_view.account_id')}
                      value={account!.id}
                      copyValue={account!.id}
                      mono
                    />
                    <DetailRow
                      label={t('accounts_view.display_name')}
                      value={account!.display_name ?? t('accounts_view.none')}
                    />
                    <DetailRow
                      label={t('wallet_list.created')}
                      value={formatDateTime(account!.created_at)}
                    />
                    <DetailRow
                      label={t('wallet_list.updated')}
                      value={formatDateTime(account!.updated_at)}
                    />
                    <DetailRow
                      label={t('accounts_view.preferences')}
                      value={
                        account!.preferences ? (
                          <Box
                            component="span"
                            sx={{
                              p: 1.5,
                              width: 1,
                              display: 'block',
                              overflow: 'auto',
                              borderRadius: 1,
                              whiteSpace: 'pre-wrap',
                              overflowWrap: 'anywhere',
                              bgcolor: 'background.neutral',
                              fontFamily: 'monospace',
                            }}
                          >
                            {formatDashboardSettings(account!.preferences.dashboard_settings)}
                          </Box>
                        ) : (
                          t('accounts_view.none')
                        )
                      }
                    />
                  </DetailCard>
                </Grid>

                <Grid size={{ xs: 12, md: 6 }}>
                  <DetailCard
                    title={t('accounts_view.login_identity')}
                    icon="solar:shield-keyhole-bold-duotone"
                    color="warning"
                  >
                    <DetailRow
                      label={t('accounts_view.provider')}
                      value={account!.identity?.provider ?? t('accounts_view.none')}
                    />
                    <DetailRow
                      label={t('accounts_view.subject')}
                      value={account!.identity?.subject ?? t('accounts_view.none')}
                      copyValue={account!.identity?.subject}
                    />
                    <DetailRow
                      label={t('accounts_view.identity_id')}
                      value={account!.identity?.id ?? t('accounts_view.none')}
                      copyValue={account!.identity?.id}
                      mono
                    />
                    <DetailRow
                      label={t('accounts_view.permissions')}
                      value={
                        account!.permissions?.length ? (
                          <Stack
                            direction="row"
                            spacing={0.75}
                            useFlexGap
                            sx={{ flexWrap: 'wrap' }}
                          >
                            {account!.permissions.map((permission) => (
                              <Chip
                                key={permission}
                                label={permission}
                                size="small"
                                variant="outlined"
                              />
                            ))}
                          </Stack>
                        ) : account!.identity?.provider === AuthProvider.OAUTH2 ? (
                          t('accounts_view.oauth_permissions')
                        ) : (
                          t('accounts_view.no_permissions')
                        )
                      }
                    />
                  </DetailCard>
                </Grid>
              </Grid>

              <Box>
                <Stack
                  direction="row"
                  spacing={2}
                  sx={{ mb: 2, alignItems: 'center', justifyContent: 'space-between' }}
                >
                  <Typography variant="h6">
                    {t('accounts_view.wallets_title', { count: accountWallets.length })}
                  </Typography>
                  <RoleBasedGuard permissions={[Permission.WRITE_WALLET]}>
                    <Button
                      size="small"
                      variant="contained"
                      startIcon={<Iconify icon="mingcute:add-line" />}
                      onClick={newWallet.onTrue}
                    >
                      {t('accounts_view.new_wallet')}
                    </Button>
                  </RoleBasedGuard>
                </Stack>

                {accountWallets.length ? (
                  <Grid container spacing={2}>
                    {accountWallets.map((wallet) => (
                      <Grid key={wallet.id} size={{ xs: 12, sm: 6, lg: 4 }}>
                        <AccountWalletCard wallet={wallet} />
                      </Grid>
                    ))}
                  </Grid>
                ) : (
                  <Card sx={{ borderRadius: 1 }}>
                    <EmptyContent title={t('accounts_view.no_wallets')} sx={{ py: 5 }} />
                  </Card>
                )}
              </Box>
            </Stack>

            <AccountEditorDrawer
              open={editor.value}
              account={account!}
              onClose={editor.onFalse}
              onSuccess={editor.onFalse}
            />

            <RegisterWalletDrawer
              key={id}
              open={newWallet.value}
              accountId={id}
              onClose={newWallet.onFalse}
              onSuccess={() => {
                mutate(endpointKeys.accounts.get(id));
                mutate(endpointKeys.accounts.list);
                mutate(endpointKeys.wallets.listOverviews);
                newWallet.onFalse();
              }}
            />

            <ConfirmDialog
              open={confirmDelete.value}
              onClose={confirmDelete.onFalse}
              title={t('accounts_view.delete_title')}
              content={t('accounts_view.delete_description', { name: accountName(account!) })}
              action={
                <Button
                  color="error"
                  variant="contained"
                  loading={isDeleting.value}
                  onClick={handleDelete}
                >
                  {t('delete')}
                </Button>
              }
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}

function AccountEditorDrawer({
  account,
  open,
  onClose,
  onSuccess,
}: {
  account: Account | null;
  open: boolean;
  onClose: VoidFunction;
  onSuccess: (account: Account) => void;
}) {
  const { t } = useTranslate();
  const [displayName, setDisplayName] = useState('');
  const [permissions, setPermissions] = useState<PermissionValue[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const isEditing = Boolean(account);
  const permissionsEditable = account?.identity?.provider !== AuthProvider.OAUTH2;

  useEffect(() => {
    if (!open) return;

    setDisplayName(account?.display_name ?? '');
    setPermissions(account?.permissions ?? []);
  }, [account, open]);

  const handleSubmit = async () => {
    setIsSubmitting(true);
    try {
      let savedAccount: Account;

      if (account) {
        const { data } = await updateAccountById<true>({
          path: { id: account.id },
          body: { display_name: displayName.trim() || null },
        });
        savedAccount = data;

        if (permissionsEditable) {
          const { data: accountWithPermissions } = await replaceAccountPermissions<true>({
            path: { id: account.id },
            body: { permissions },
          });
          savedAccount = accountWithPermissions;
        }
      } else {
        const { data } = await createAccount<true>({
          body: {
            display_name: displayName.trim() || null,
            permissions,
          },
        });
        savedAccount = data;
      }

      await Promise.all([
        mutate(endpointKeys.accounts.list),
        mutate(endpointKeys.accounts.get(savedAccount.id)),
      ]);
      toast.success(t(isEditing ? 'accounts_view.update_success' : 'accounts_view.create_success'));
      onSuccess(savedAccount);
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const togglePermission = (permission: PermissionValue) => {
    setPermissions((current) =>
      current.includes(permission)
        ? current.filter((value) => value !== permission)
        : [...current, permission]
    );
  };

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: { xs: 1, sm: 520 }, maxWidth: 1 } } }}
    >
      <Stack
        direction="row"
        sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
      >
        <Typography variant="h6">
          {t(isEditing ? 'accounts_view.edit_account' : 'accounts_view.new_account')}
        </Typography>
        <IconButton onClick={onClose} aria-label={t('close')}>
          <Iconify icon="mingcute:close-line" />
        </IconButton>
      </Stack>

      <Divider />

      <Stack spacing={2.5} sx={{ p: 3, overflowY: 'auto', flex: 1 }}>
        <TextField
          autoFocus
          fullWidth
          label={t('accounts_view.display_name')}
          value={displayName}
          onChange={(event) => setDisplayName(event.target.value)}
        />

        <Divider />

        {permissionsEditable ? (
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              {t('accounts_view.permissions')}
            </Typography>
            <FormGroup
              sx={{
                display: 'grid',
                gridTemplateColumns: { xs: '1fr', sm: 'repeat(2, minmax(0, 1fr))' },
              }}
            >
              {permissionOptions.map((permission) => (
                <FormControlLabel
                  key={permission}
                  label={permission}
                  control={
                    <Checkbox
                      checked={permissions.includes(permission)}
                      onChange={() => togglePermission(permission)}
                    />
                  }
                />
              ))}
            </FormGroup>
          </Box>
        ) : (
          <Alert severity="info" variant="outlined">
            {t('accounts_view.oauth_permissions')}
          </Alert>
        )}
      </Stack>

      <Divider />

      <Stack direction="row" spacing={1.5} sx={{ justifyContent: 'flex-end', p: 3 }}>
        <Button color="inherit" variant="outlined" onClick={onClose}>
          {t('cancel')}
        </Button>
        <Button variant="contained" loading={isSubmitting} onClick={handleSubmit}>
          {t(isEditing ? 'save' : 'accounts_view.create_account')}
        </Button>
      </Stack>
    </Drawer>
  );
}
