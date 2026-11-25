import type { Asset, Balance } from 'src/lib/swissknife';

const MAX_WALLET_UNITS = BigInt('9223372036854775807');

export function creditDenomination(asset: Asset) {
  if (asset.protocol === 'bitcoin' && asset.asset_ref === 'native' && asset.decimals >= 8) {
    return { decimals: asset.decimals - 8, symbol: 'sat' };
  }
  return { decimals: asset.decimals, symbol: asset.display_ticker };
}

export function parseCreditAmount(input: string, decimals: number): bigint | undefined {
  const match = /^(\d+)(?:[.,](\d+))?$/.exec(input.trim());
  if (!match || !Number.isInteger(decimals) || decimals < 0 || decimals > 255) return undefined;
  const fraction = match[2] ?? '';
  if (fraction.length > decimals || match[1].length > 20) return undefined;
  const units = BigInt(match[1] + fraction.padEnd(decimals, '0'));
  return units > BigInt(0) && units <= MAX_WALLET_UNITS ? units : undefined;
}

export function availableUnits(balance: Balance): bigint | undefined {
  if (balance.available_units != null) return BigInt(balance.available_units);
  return Number.isSafeInteger(balance.available_msat) ? BigInt(balance.available_msat) : undefined;
}

export function projectedCreditBalance(available?: bigint, amount?: bigint) {
  if (available == null || amount == null) return undefined;
  const sum = available + amount;
  return sum <= MAX_WALLET_UNITS ? sum : undefined;
}

export function displayCreditUnits(units: bigint, decimals: number, locale: string) {
  const digits = units
    .toString()
    .replace('-', '')
    .padStart(decimals + 1, '0');
  const integer = decimals ? digits.slice(0, -decimals) : digits;
  const fraction = decimals ? digits.slice(-decimals).replace(/0+$/, '') : '';
  const separator =
    new Intl.NumberFormat(locale).formatToParts(1.1).find((p) => p.type === 'decimal')?.value ??
    '.';
  return (
    (units < BigInt(0) ? '-' : '') +
    BigInt(integer).toLocaleString(locale) +
    (fraction ? separator + fraction : '')
  );
}
