import type { AutocompleteProps } from '@mui/material/Autocomplete';

import { Controller, useFormContext } from 'react-hook-form';

import TextField from '@mui/material/TextField';
import { CircularProgress } from '@mui/material';
import Autocomplete from '@mui/material/Autocomplete';

// ----------------------------------------------------------------------

export type AutocompleteBaseProps = Omit<AutocompleteProps<any, boolean, boolean, boolean>, 'renderInput'>;

export type RHFAutocompleteProps = AutocompleteBaseProps & {
  name: string;
  label?: string;
  placeholder?: string;
  hiddenLabel?: boolean;
  helperText?: React.ReactNode;
  loading?: boolean;
  selectedField?: string;
};

export function RHFAutocomplete({
  name,
  label,
  helperText,
  hiddenLabel,
  loading,
  selectedField = 'id',
  placeholder,
  ...other
}: RHFAutocompleteProps) {
  const { control, setValue } = useFormContext();

  return (
    <Controller
      name={name}
      control={control}
      render={({ field, fieldState: { error } }) => (
        <Autocomplete
          {...field}
          id={`rhf-autocomplete-${name}`}
          onChange={(event, newValue) => setValue(name, newValue, { shouldValidate: true })}
          renderInput={(params) => (
            <TextField
              {...params}
              label={label}
              placeholder={placeholder}
              error={!!error}
              helperText={error ? error?.message : helperText}
              inputProps={{ ...params.inputProps, autoComplete: 'new-password' }}
              InputProps={{
                ...params.InputProps,
                endAdornment: (
                  <>
                    {loading ? <CircularProgress color="inherit" size={20} /> : null}
                    {params.InputProps.endAdornment}
                  </>
                ),
              }}
            />
          )}
          {...other}
        />
      )}
    />
  );
}
