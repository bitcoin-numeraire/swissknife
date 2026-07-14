import type { CreateWalletRequest } from 'src/lib/swissknife';

import { useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

import { Stack } from '@mui/material';
import Button from '@mui/material/Button';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { registerWallet } from 'src/lib/swissknife';
import { useAccountContext } from 'src/contexts/account';
import { zCreateWalletRequest } from 'src/lib/swissknife/zod.gen';

import { toast } from 'src/components/snackbar';
import { Form, RHFTextField } from 'src/components/hook-form';

// ----------------------------------------------------------------------

export type NewWalletFormProps = {
  accountId?: string;
  onSuccess: VoidFunction;
};

export function RegisterWalletForm({ accountId, onSuccess }: NewWalletFormProps) {
  const { t } = useTranslate();
  const { activeWalletId, wallets, walletsLoading } = useAccountContext();
  const activeAssetId = wallets.find((wallet) => wallet.id === activeWalletId)?.asset_id ?? '';

  const methods = useForm<CreateWalletRequest>({
    resolver: zodResolver(zCreateWalletRequest),
    defaultValues: {
      account_id: accountId ?? '',
      asset_id: activeAssetId,
    },
  });

  const {
    reset,
    setValue,
    handleSubmit,
    formState: { isSubmitting },
    watch,
  } = methods;

  const selectedAccountId = watch('account_id');

  useEffect(() => {
    setValue('asset_id', activeAssetId, { shouldValidate: true });
  }, [activeAssetId, setValue]);

  const onSubmit = async (body: CreateWalletRequest) => {
    try {
      await registerWallet({ body });
      toast.success(t('register_wallet.success_wallet_registration'));
      reset({ account_id: accountId ?? '', asset_id: activeAssetId });
      onSuccess();
    } catch (error) {
      handleActionError(error);
    }
  };

  return (
    <Form methods={methods} onSubmit={handleSubmit(onSubmit)}>
      <Stack spacing={3}>
        <RHFTextField
          variant="outlined"
          name="account_id"
          label={t('register_wallet.account_id')}
          disabled={Boolean(accountId)}
        />
        <Button
          type="submit"
          variant="contained"
          color="inherit"
          size="large"
          loading={isSubmitting}
          disabled={!selectedAccountId || !activeAssetId || walletsLoading || isSubmitting}
        >
          {t('register')}
        </Button>
      </Stack>
    </Form>
  );
}
