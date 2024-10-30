import type { ListWalletsResponse } from 'src/lib/swissknife';

import { useState } from 'react';

import { useBoolean } from 'src/hooks/use-boolean';

import { truncateText } from 'src/utils/format-string';

import { listWallets } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

import { RHFAutocomplete } from './rhf-autocomplete';

// ----------------------------------------------------------------------

export function RHFWalletSelect() {
  const [options, setOptions] = useState<ListWalletsResponse>([]);
  const loading = useBoolean();

  const fetchOptions = async () => {
    loading.onTrue();
    try {
      const { error, data } = await listWallets();
      if (error) throw new Error(error.reason);
      setOptions(data);
    } catch (error) {
      toast.error(error.message);
    } finally {
      loading.onFalse();
    }
  };

  return (
    <RHFAutocomplete
      name="wallet"
      label="Select wallet"
      options={options.filter((option) => option.ln_address == null)}
      loading={loading.value}
      getOptionLabel={(option) => `${option.user_id} (${option.id})`}
      isOptionEqualToValue={(option, value) => option.id === value.id}
      onOpen={fetchOptions}
      renderOption={(props, option) => (
        <li {...props} key={option.id}>
          {option.user_id} ({truncateText(option.id, 15)})
        </li>
      )}
    />
  );
}
