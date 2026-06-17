import type { FieldError } from 'react-hook-form';
import type { TextFieldProps } from '@mui/material/TextField';
import type { AutocompleteProps, AutocompleteRenderInputParams } from '@mui/material/Autocomplete';

import { merge } from 'es-toolkit';
import { useCallback } from 'react';
import { Controller, useFormContext } from 'react-hook-form';

import TextField from '@mui/material/TextField';
import Autocomplete from '@mui/material/Autocomplete';

// ----------------------------------------------------------------------

type Multiple = boolean | undefined;
type DisableClearable = boolean | undefined;
type FreeSolo = boolean | undefined;

type ExcludedProps = 'renderInput';

export type AutocompleteBaseProps = Omit<
  AutocompleteProps<any, Multiple, DisableClearable, FreeSolo>,
  ExcludedProps
>;

export type RHFAutocompleteProps = AutocompleteBaseProps & {
  name: string;
  label?: string;
  placeholder?: string;
  helperText?: React.ReactNode;
  slotProps?: AutocompleteBaseProps['slotProps'] & {
    textField?: Partial<TextFieldProps>;
  };
};

export function RHFAutocomplete({
  name,
  label,
  slotProps,
  helperText,
  placeholder,
  ...other
}: RHFAutocompleteProps) {
  const { control, setValue } = useFormContext();

  const { textField, ...otherSlotProps } = slotProps ?? {};

  const renderInput = useCallback(
    (params: AutocompleteRenderInputParams, error?: FieldError) => {
      const { slotProps: systemSlotProps, ...otherSystemProps } = params;
      const { slotProps: externalTextFieldSlotProps, ...otherTextFieldProps } = textField || {};

      const mergedSlotProps: TextFieldProps['slotProps'] = merge(
        systemSlotProps,
        externalTextFieldSlotProps ?? {}
      );

      return (
        <TextField
          {...otherSystemProps}
          label={label}
          placeholder={placeholder}
          error={!!error}
          helperText={error?.message ?? helperText}
          {...otherTextFieldProps}
          slotProps={{
            ...mergedSlotProps,
            htmlInput: {
              ...mergedSlotProps.htmlInput,
              autoComplete: 'new-password',
            },
          }}
        />
      );
    },
    [helperText, label, placeholder, textField]
  );

  return (
    <Controller
      name={name}
      control={control}
      render={({ field, fieldState }) => (
        <Autocomplete
          {...field}
          id={`${name}-rhf-autocomplete`}
          onChange={(event, newValue) => setValue(name, newValue, { shouldValidate: true })}
          renderInput={(params) => renderInput(params, fieldState.error)}
          slotProps={{
            ...otherSlotProps,
            chip: {
              size: 'small',
              variant: 'soft',
              ...otherSlotProps?.chip,
            },
          }}
          {...other}
        />
      )}
    />
  );
}
