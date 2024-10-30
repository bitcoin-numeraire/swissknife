'use client';

import type { CurrencyValue } from 'src/types/currency';
import type { IconButtonProps } from '@mui/material/IconButton';

import { m } from 'framer-motion';
import { useCallback } from 'react';

import MenuList from '@mui/material/MenuList';
import MenuItem from '@mui/material/MenuItem';
import IconButton from '@mui/material/IconButton';

import { Label } from 'src/components/label';
import { varHover } from 'src/components/animate';
import { useSettingsContext } from 'src/components/settings';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

// ----------------------------------------------------------------------

export type CurrencyPopoverProps = IconButtonProps & {
  data?: Array<CurrencyValue>;
};

export function CurrencyPopover({ data = [], sx, ...other }: CurrencyPopoverProps) {
  const popover = usePopover();
  const { currency, onUpdateField } = useSettingsContext();

  const handleChangeCurrency = useCallback(
    (newCurr: CurrencyValue) => {
      if (newCurr !== currency) {
        onUpdateField('currency', newCurr);
      }
      popover.onClose();
    },
    [popover, currency, onUpdateField]
  );

  return (
    <>
      <IconButton
        component={m.button}
        whileTap="tap"
        whileHover="hover"
        variants={varHover(1.05)}
        onClick={popover.onOpen}
        sx={{ p: 0, ...sx }}
        {...other}
      >
        <Label color="default" variant="filled" sx={{ p: 1, cursor: 'pointer' }}>
          {currency}
        </Label>
      </IconButton>

      <CustomPopover open={popover.open} anchorEl={popover.anchorEl} onClose={popover.onClose}>
        <MenuList sx={{ width: 'auto' }}>
          {data?.map((option) => (
            <MenuItem key={option} selected={option === currency} onClick={() => handleChangeCurrency(option)}>
              {option}
            </MenuItem>
          ))}
        </MenuList>
      </CustomPopover>
    </>
  );
}
