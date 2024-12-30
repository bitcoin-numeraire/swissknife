'use client';

import type { CurrencyValue } from 'src/types/currency';
import type { IconButtonProps } from '@mui/material/IconButton';

import { m } from 'framer-motion';
import { useCallback } from 'react';
import { usePopover } from 'minimal-shared/hooks';

import MenuList from '@mui/material/MenuList';
import MenuItem from '@mui/material/MenuItem';
import IconButton from '@mui/material/IconButton';

import { Label } from 'src/components/label';
import { useSettingsContext } from 'src/components/settings';
import { CustomPopover } from 'src/components/custom-popover';
import { varTap, varHover, transitionTap } from 'src/components/animate';

// ----------------------------------------------------------------------

export type CurrencyPopoverProps = IconButtonProps & {
  data?: Array<CurrencyValue>;
};

export function CurrencyPopover({ data = [], sx, ...other }: CurrencyPopoverProps) {
  const { open, anchorEl, onClose, onOpen } = usePopover();
  const { state, setState } = useSettingsContext();
  const { currency } = state;

  const handleChangeCurrency = useCallback(
    (newCurr: CurrencyValue) => {
      if (newCurr !== currency) {
        setState({ currency: newCurr });
      }
      onClose();
    },
    [setState, onClose, currency]
  );

  const renderMenuList = () => (
    <CustomPopover open={open} anchorEl={anchorEl} onClose={onClose}>
      <MenuList sx={{ width: 160, minHeight: 72 }}>
        {data?.map((option) => (
          <MenuItem
            key={option}
            selected={option === currency}
            onClick={() => handleChangeCurrency(option)}
          >
            {option}
          </MenuItem>
        ))}
      </MenuList>
    </CustomPopover>
  );

  return (
    <>
      <IconButton
        component={m.button}
        whileTap={varTap(0.96)}
        whileHover={varHover(1.04)}
        transition={transitionTap()}
        aria-label="Currencies button"
        onClick={onOpen}
        sx={[
          (theme) => ({
            p: 0,
            width: 40,
            height: 40,
            ...(open && { bgcolor: theme.vars.palette.action.selected }),
          }),
          ...(Array.isArray(sx) ? sx : [sx]),
        ]}
        {...other}
      >
        <Label color="default" variant="filled" sx={{ p: 1, cursor: 'pointer' }}>
          {currency}
        </Label>
      </IconButton>

      {renderMenuList()}
    </>
  );
}
