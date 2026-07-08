import type { Wallet } from 'src/lib/swissknife';

import { useFormContext } from 'react-hook-form';

import { Box } from '@mui/material';

import { displayLnAddress } from 'src/utils/lnurl';
import { handleActionError } from 'src/utils/errors';

import { useListWallets } from 'src/actions/wallet';

import { RHFAutocomplete } from 'src/components/hook-form';

// ----------------------------------------------------------------------

type WalletSelectProps = {
  name?: 'account_id' | 'wallet_id';
  linkedAccountName?: 'account_id';
};

function walletFieldValue(wallet: Wallet, name: WalletSelectProps['name']) {
  return name === 'wallet_id' ? wallet.id : wallet.account_id;
}

function walletOptionLabel(wallet: Wallet) {
  if (wallet.ln_address?.username) return displayLnAddress(wallet.ln_address.username);

  return wallet.label ?? wallet.account_id;
}

export function WalletSelectDropdown({ name = 'wallet_id', linkedAccountName }: WalletSelectProps) {
  const { wallets, walletsError, walletsLoading } = useListWallets();
  const { setValue } = useFormContext();

  if (walletsError) {
    handleActionError(walletsError);
  }

  if (walletsLoading) {
    return <Box>Loading...</Box>;
  }

  return (
    wallets && (
      <RHFAutocomplete
        name={name}
        label="Select wallet"
        options={wallets.map((wallet) => walletFieldValue(wallet, name))}
        getOptionLabel={(option: string) => {
          const wallet = wallets.find((item) => walletFieldValue(item, name) === option);

          return wallet ? walletOptionLabel(wallet) : option;
        }}
        onChange={(_, newValue) => {
          setValue(name, newValue, { shouldValidate: true });
          if (linkedAccountName) {
            const wallet = wallets.find((item) => walletFieldValue(item, name) === newValue);
            setValue(linkedAccountName, wallet?.account_id ?? null, { shouldValidate: true });
          }
        }}
      />
    )
  );
}
