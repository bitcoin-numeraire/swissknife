import type { CreateWalletRequest } from 'src/lib/swissknife';

import { mutate } from 'swr';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import Button from '@mui/material/Button';
import { Stack, Alert } from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { useListAssets } from 'src/actions/asset';
import { registerWallet } from 'src/lib/swissknife';
import { useGetAccount } from 'src/actions/account';
import { zCreateWalletRequest } from 'src/lib/swissknife/zod.gen';

import { toast } from 'src/components/snackbar';
import { AccountSelect } from 'src/components/account';
import { Form, RHFTextField } from 'src/components/hook-form';

import { AssetSelect } from './asset-select';

// ----------------------------------------------------------------------

export type NewWalletFormProps = {
  accountId?: string;
  onSuccess: VoidFunction;
};

export function RegisterWalletForm({ accountId, onSuccess }: NewWalletFormProps) {
  const { t } = useTranslate();
  const catalog = useListAssets();

  const methods = useForm<CreateWalletRequest>({
    resolver: zodResolver(zCreateWalletRequest),
    defaultValues: {
      account_id: accountId ?? '',
      asset_id: '',
    },
  });

  const {
    reset,
    setValue,
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const selectedAccountId = watch('account_id') ?? '';
  const selectedAssetId = watch('asset_id');
  const { account, accountLoading, accountError } = useGetAccount(selectedAccountId);
  const owned = new Set(account?.wallets.map((wallet) => wallet.asset_id));
  const available = catalog.data?.filter((asset) => !owned.has(asset.id)) ?? [];
  const loading = accountLoading || catalog.isLoading;
  const failed = Boolean(accountError || catalog.error);
  const ready = account?.id === selectedAccountId && !loading && !failed;
  const canRegister = ready && available.some((asset) => asset.id === selectedAssetId);

  const onSubmit = async (body: CreateWalletRequest) => {
    if (!canRegister) return;
    try {
      await registerWallet<true>({ body });
      void mutate(endpointKeys.accounts.get(selectedAccountId));
      void mutate(endpointKeys.accounts.list);
      toast.success(t('register_wallet.success_wallet_registration'));
      reset({ account_id: accountId ?? '', asset_id: '' });
      onSuccess();
    } catch (error) {
      handleActionError(error);
    }
  };

  return (
    <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
      <Stack spacing={3}>
        {accountId ? (
          <RHFTextField
            variant="outlined"
            name="account_id"
            label={t('register_wallet.account_id')}
            disabled
          />
        ) : (
          <AccountSelect onAccountChange={() => setValue('asset_id', '')} />
        )}
        {failed && <Alert severity="error">{t('register_wallet.asset_load_error')}</Alert>}
        <AssetSelect
          assets={ready ? available : []}
          disabled={!ready || !available.length}
          loading={loading}
          helperText={
            loading
              ? t('register_wallet.loading_assets')
              : ready && !available.length
                ? t('register_wallet.no_assets')
                : undefined
          }
        />
        <Button
          type="submit"
          variant="contained"
          color="inherit"
          size="large"
          loading={isSubmitting}
          disabled={!canRegister || isSubmitting}
        >
          {t('register')}
        </Button>
      </Stack>
    </Form>
  );
}
