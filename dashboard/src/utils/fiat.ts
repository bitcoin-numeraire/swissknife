import type { IFiatPrices } from 'src/types/bitcoin';
import type { CurrencyValue } from 'src/types/currency';

const SATS_PER_BITCOIN = 100000000;

export function satsToFiat(amountSats: number, fiatPrices: IFiatPrices, currency: CurrencyValue = 'USD'): number {
  return (amountSats / SATS_PER_BITCOIN) * fiatPrices[currency];
}
