import { Box } from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useListWallets } from 'src/actions/wallet';

import { RHFAutocomplete } from 'src/components/hook-form';

// ----------------------------------------------------------------------

type WalletSelectProps = {
  name?: 'user_id' | 'wallet_id';
};

export function WalletSelectDropdown({ name = 'wallet_id' }: WalletSelectProps) {
  const { wallets, walletsError, walletsLoading } = useListWallets();
  const selectorField = name === 'wallet_id' ? 'id' : name;

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
        options={wallets.map((wallet) => wallet[selectorField])}
        getOptionLabel={(option: string) =>
          wallets.find((w) => w[selectorField] === option)!.user_id
        }
      />
    )
  );
}
