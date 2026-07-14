'use client';

import type { Key, HTMLAttributes } from 'react';
import type { Account } from 'src/lib/swissknife';

import { useFormContext } from 'react-hook-form';

import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { createFilterOptions } from '@mui/material/Autocomplete';

import { useTranslate } from 'src/locales';
import { useListAccounts } from 'src/actions/account';

import { RHFAutocomplete } from 'src/components/hook-form';

// ----------------------------------------------------------------------

const filterAccounts = createFilterOptions<Account>({
  stringify: (account) =>
    [account.id, account.display_name, account.identity?.provider, account.identity?.subject]
      .filter(Boolean)
      .join(' '),
});

function compactId(id: string) {
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

function accountName(account: Account) {
  return account.display_name?.trim() || account.identity?.subject;
}

function accountOptionLabel(account: Account) {
  const name = accountName(account);
  return name ? `${name} · ${compactId(account.id)}` : account.id;
}

type Props = {
  name?: 'account_id';
};

export function AccountSelect({ name = 'account_id' }: Props) {
  const { t } = useTranslate();
  const { watch, setValue } = useFormContext();
  const { accounts, accountsLoading, accountsError } = useListAccounts();
  const accountOptions = accounts ?? [];
  const selectedAccount = accountOptions.find((account) => account.id === watch(name)) ?? null;

  if (accountsError) {
    return <Alert severity="warning">{t('account_selector.load_error')}</Alert>;
  }

  return (
    <RHFAutocomplete
      name={name}
      label={t('account_selector.label')}
      options={accountOptions}
      value={selectedAccount}
      loading={accountsLoading}
      loadingText={t('account_selector.loading')}
      noOptionsText={t('account_selector.empty')}
      filterOptions={filterAccounts}
      getOptionLabel={(option: Account) => accountOptionLabel(option)}
      isOptionEqualToValue={(option: Account, value: Account) => option.id === value.id}
      onChange={(_, account: Account | null) =>
        setValue(name, account?.id ?? null, { shouldValidate: true })
      }
      renderOption={(props: HTMLAttributes<HTMLLIElement> & { key: Key }, account: Account) => {
        const { key, ...optionProps } = props;
        const identity = account.identity;
        const metadata = identity
          ? [identity.provider.toUpperCase(), account.display_name ? identity.subject : null]
          : [t('accounts_view.no_identity')];

        return (
          <li key={key} {...optionProps}>
            <Stack sx={{ minWidth: 0 }}>
              <Typography variant="subtitle2" noWrap>
                {accountName(account) || account.id}
              </Typography>
              <Typography variant="caption" color="text.secondary" noWrap>
                {[...metadata, compactId(account.id)].filter(Boolean).join(' · ')}
              </Typography>
            </Stack>
          </li>
        );
      }}
    />
  );
}
