import type { Key, HTMLAttributes } from 'react';
import type { Asset, CreateWalletRequest } from 'src/lib/swissknife';

import { useFormContext } from 'react-hook-form';

import Stack from '@mui/material/Stack';
import Avatar from '@mui/material/Avatar';
import ListItemText from '@mui/material/ListItemText';

import { useTranslate } from 'src/locales';

import { RHFAutocomplete } from 'src/components/hook-form';

type Props = {
  assets: Asset[];
  disabled: boolean;
  loading: boolean;
  helperText?: string;
};

export function AssetSelect({ assets, disabled, loading, helperText }: Props) {
  const { t } = useTranslate();
  const { watch, setValue } = useFormContext<CreateWalletRequest>();
  const assetId = watch('asset_id');

  return (
    <RHFAutocomplete
      name="asset_id"
      label={t('wallet_details.asset')}
      disabled={disabled}
      loading={loading}
      loadingText={t('register_wallet.loading_assets')}
      noOptionsText={t('register_wallet.no_matches')}
      helperText={helperText}
      options={assets}
      value={assets.find((asset) => asset.id === assetId) ?? null}
      getOptionKey={(asset: Asset) => asset.id}
      getOptionLabel={(asset: Asset) =>
        [asset.name || asset.code, asset.display_ticker, asset.network].join(' · ')
      }
      isOptionEqualToValue={(option: Asset, value: Asset) => option.id === value.id}
      onChange={(_, asset: Asset | null) =>
        setValue('asset_id', asset?.id ?? '', { shouldValidate: true })
      }
      renderOption={(props: HTMLAttributes<HTMLLIElement> & { key: Key }, asset: Asset) => {
        const { key, ...optionProps } = props;
        return (
          <li key={key} {...optionProps}>
            <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
              <Avatar
                sx={{ typography: 'caption', bgcolor: 'background.neutral', color: 'text.primary' }}
              >
                {asset.display_ticker}
              </Avatar>
              <ListItemText
                primary={asset.name || asset.code}
                secondary={[asset.code, asset.protocol, asset.network].join(' · ')}
              />
            </Stack>
          </li>
        );
      }}
    />
  );
}
