'use client';

import type { TypographyProps } from '@mui/material';

import React from 'react';

import { Tooltip, Typography } from '@mui/material';

import { fSats } from 'src/utils/format-number';

import { useSettingsContext } from 'src/components/settings';

interface Props extends TypographyProps {
  amountMSats: number;
  placement?: 'top-start' | 'top' | 'bottom' | 'left' | 'right';
  showMillisatsTooltip?: boolean;
  children?: React.ReactNode;
}

export function SatsWithIcon({
  amountMSats,
  placement = 'top-start',
  showMillisatsTooltip = false,
  children,
  variant,
  ...other
}: Props) {
  const { state } = useSettingsContext();
  const hasSubSat = amountMSats % 1000 !== 0;
  const sats = amountMSats / 1000;
  const formattedAmount = fSats(sats, {
    maximumFractionDigits: showMillisatsTooltip && hasSubSat ? 3 : 0,
  });
  const displayUnit = state.displayUnit ?? 'bip177';
  const hideBalances = state.hideBalances ?? false;
  const content =
    displayUnit === 'sats' ? (
      <>
        {hideBalances ? '••••' : formattedAmount} <span style={{ opacity: 0.65 }}>sats</span>
      </>
    ) : (
      <>
        <span style={{ opacity: 0.65, marginRight: 2 }}>₿</span>
        {hideBalances ? '••••' : formattedAmount}
      </>
    );

  const amount = (
    <Typography variant={variant || 'inherit'} {...other}>
      {content}
      {children}
    </Typography>
  );

  if (!showMillisatsTooltip || hideBalances) return amount;

  return (
    <Tooltip title={`${fSats(amountMSats)} mSats`} placement={placement} arrow>
      {amount}
    </Tooltip>
  );
}
