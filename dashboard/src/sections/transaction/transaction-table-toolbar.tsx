import type { IDatePickerControl } from 'src/types/common';
import type { SelectChangeEvent } from '@mui/material/Select';
import type { UseSetStateReturn } from 'src/hooks/use-set-state';
import type { ITransactionTableFilters } from 'src/types/transaction';

import { useCallback } from 'react';

import Stack from '@mui/material/Stack';
import { MenuList } from '@mui/material';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Checkbox from '@mui/material/Checkbox';
import TextField from '@mui/material/TextField';
import InputLabel from '@mui/material/InputLabel';
import IconButton from '@mui/material/IconButton';
import FormControl from '@mui/material/FormControl';
import OutlinedInput from '@mui/material/OutlinedInput';
import InputAdornment from '@mui/material/InputAdornment';
import { DatePicker } from '@mui/x-date-pickers/DatePicker';
import { formHelperTextClasses } from '@mui/material/FormHelperText';

import { useTranslate } from 'src/locales';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

type Props = {
  dateError: boolean;
  onResetPage: () => void;
  filters: UseSetStateReturn<ITransactionTableFilters>;
  invoiceLedgerOptions: string[];
};

export function TransactionTableToolbar({ filters, onResetPage, dateError, invoiceLedgerOptions }: Props) {
  const { t } = useTranslate();
  const popover = usePopover();

  const handleFilterName = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      onResetPage();
      filters.setState({ name: event.target.value });
    },
    [filters, onResetPage]
  );

  const handleFilterLedger = useCallback(
    (event: SelectChangeEvent<string[]>) => {
      const newValue = typeof event.target.value === 'string' ? event.target.value.split(',') : event.target.value;

      onResetPage();
      filters.setState({ ledger: newValue });
    },
    [filters, onResetPage]
  );

  const handleFilterStartDate = useCallback(
    (newValue: IDatePickerControl) => {
      onResetPage();
      filters.setState({ startDate: newValue });
    },
    [filters, onResetPage]
  );

  const handleFilterEndDate = useCallback(
    (newValue: IDatePickerControl) => {
      onResetPage();
      filters.setState({ endDate: newValue });
    },
    [filters, onResetPage]
  );

  return (
    <>
      <Stack
        spacing={2}
        alignItems={{ xs: 'flex-end', md: 'center' }}
        direction={{
          xs: 'column',
          md: 'row',
        }}
        sx={{
          p: 2.5,
          pr: { xs: 2.5, md: 1 },
        }}
      >
        <FormControl
          sx={{
            flexShrink: 0,
            width: { xs: 1, md: 180 },
          }}
        >
          <InputLabel>{t('transaction_details.ledger')}</InputLabel>

          <Select
            multiple
            value={filters.state.ledger}
            onChange={handleFilterLedger}
            input={<OutlinedInput label="Ledger" />}
            renderValue={(selected) => selected.map((value) => value).join(', ')}
            inputProps={{ id: 'transaction-filter-ledger-select-label' }}
            sx={{ textTransform: 'capitalize' }}
          >
            {invoiceLedgerOptions.map((option) => (
              <MenuItem key={option} value={option} sx={{ textTransform: 'capitalize' }}>
                <Checkbox disableRipple size="small" checked={filters.state.ledger.includes(option)} />
                {option.toLowerCase()}
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        <DatePicker
          label={t('start_date')}
          value={filters.state.startDate}
          onChange={handleFilterStartDate}
          slotProps={{ textField: { fullWidth: true } }}
          sx={{
            maxWidth: { md: 180 },
          }}
        />

        <DatePicker
          label={t('end_date')}
          value={filters.state.endDate}
          onChange={handleFilterEndDate}
          slotProps={{
            textField: {
              fullWidth: true,
              error: dateError,
              helperText: dateError && t('end_date_error'),
            },
          }}
          sx={{
            maxWidth: { md: 180 },
            [`& .${formHelperTextClasses.root}`]: {
              position: { md: 'absolute' },
              bottom: { md: -40 },
            },
          }}
        />

        <Stack direction="row" alignItems="center" spacing={2} flexGrow={1} sx={{ width: 1 }}>
          <TextField
            fullWidth
            value={filters.state.name}
            onChange={handleFilterName}
            placeholder={t('transaction_table_toolbar.search_placeholder')}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <Iconify icon="eva:search-fill" sx={{ color: 'text.disabled' }} />
                </InputAdornment>
              ),
            }}
          />

          <IconButton onClick={popover.onOpen}>
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </Stack>
      </Stack>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        <MenuList>
          <MenuItem
            onClick={() => {
              popover.onClose();
              toast.info(t('coming_soon'));
            }}
          >
            <Iconify icon="solar:printer-minimalistic-bold" />
            {t('print')}
          </MenuItem>

          <MenuItem
            onClick={() => {
              popover.onClose();
              toast.info(t('coming_soon'));
            }}
          >
            <Iconify icon="solar:export-bold" />
            {t('export')}
          </MenuItem>
        </MenuList>
      </CustomPopover>
    </>
  );
}
