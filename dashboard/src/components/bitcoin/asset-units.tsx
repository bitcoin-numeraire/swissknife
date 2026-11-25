'use client';

import type { TypographyProps } from '@mui/material';
import type { Asset } from 'src/lib/swissknife';

import Typography from '@mui/material/Typography';

import { creditDenomination, displayCreditUnits } from 'src/utils/credit-amount';

import { useTranslate } from 'src/locales';

import { useSettingsContext } from 'src/components/settings';

export function AssetUnits({
  units,
  asset,
  ...props
}: TypographyProps & { units: string; asset?: Asset | null }) {
  const { t, currentLang } = useTranslate();
  const { state } = useSettingsContext();
  const denomination = asset && creditDenomination(asset);
  const exact = state.hideBalances
    ? '••••'
    : displayCreditUnits(BigInt(units), denomination?.decimals ?? 0, currentLang.numberFormat.code);
  const label = denomination?.symbol ?? t('wallet_credit.asset_units');
  return (
    <Typography component="span" variant="inherit" {...props}>
      {denomination?.symbol === 'sat' && state.displayUnit !== 'sats'
        ? '₿' + exact
        : exact + ' ' + label}
    </Typography>
  );
}
