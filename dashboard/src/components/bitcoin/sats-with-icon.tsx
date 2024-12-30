import type { TypographyProps } from '@mui/material';

import React from 'react';

import { Tooltip, Typography } from '@mui/material';

import { fSats } from 'src/utils/format-number';

interface Props extends TypographyProps {
  amountMSats: number;
  placement?: 'top-start' | 'top' | 'bottom' | 'left' | 'right';
  children?: React.ReactNode;
}

export function SatsWithIcon({
  amountMSats,
  placement = 'top-start',
  children,
  variant,
  ...other
}: Props) {
  return (
    <Tooltip title={`${amountMSats} mSats`} placement={placement} arrow>
      <Typography variant={variant || 'inherit'} {...other}>
        {fSats(amountMSats / 1000)}
        <i className="fak fa-light" />
        {children}
      </Typography>
    </Tooltip>
  );
}
