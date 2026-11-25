import type { Balance } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import {
  availableUnits,
  parseCreditAmount,
  displayCreditUnits,
  projectedCreditBalance,
} from './credit-amount';

describe('asset credit amounts', () => {
  it('round-trips a fractional satoshi above the JavaScript safe integer range', () => {
    const units = BigInt('9007199254740993');
    expect(parseCreditAmount('9007199254740,993', 3)).toBe(units);
    expect(displayCreditUnits(units, 3, 'en-US')).toBe('9,007,199,254,740.993');
    expect(displayCreditUnits(BigInt(100), 0, 'en-US')).toBe('100');
  });
  it('rejects zero, sub-unit fractions, scientific notation, and storage overflow', () => {
    for (const input of ['-1', '0', '0.0001', '3e2', '9223372036854775.808']) {
      expect(parseCreditAmount(input, 3)).toBeUndefined();
    }
    expect(projectedCreditBalance(BigInt('9223372036854775807'), BigInt(1))).toBeUndefined();
  });
  it('only previews older balance payloads when their amount is exactly representable', () => {
    const balance = { available_msat: 9_007_199_254_740_992 } as Balance;
    expect(availableUnits(balance)).toBeUndefined();
    expect(availableUnits({ ...balance, available_units: '9007199254740993' })).toBe(
      BigInt('9007199254740993')
    );
    expect(availableUnits({ ...balance, available_msat: 100 })).toBe(BigInt(100));
  });
});
