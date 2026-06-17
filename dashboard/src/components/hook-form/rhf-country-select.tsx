import type { CountrySelectProps } from '../country-select';

import { Controller, useFormContext } from 'react-hook-form';

import { CountrySelect } from '../country-select';

// ----------------------------------------------------------------------

export type RHFCountrySelectProps = CountrySelectProps & {
  name: string;
};

export function RHFCountrySelect({ name, helperText, ...other }: RHFCountrySelectProps) {
  const { control, setValue } = useFormContext();

  return (
    <Controller
      name={name}
      control={control}
      render={({ field, fieldState: { error } }) => (
        <CountrySelect
          id={`${name}-rhf-country-select`}
          value={field.value}
          onChange={(event, newValue) => setValue(name, newValue, { shouldValidate: true })}
          error={!!error}
          helperText={error?.message ?? helperText}
          {...other}
        />
      )}
    />
  );
}
