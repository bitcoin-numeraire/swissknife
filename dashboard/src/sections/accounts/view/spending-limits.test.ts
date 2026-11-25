import { it, expect } from 'vitest';

import { editLimits, parseLimits, usedPercent } from './spending-limits';

it('round trips exact millisatoshis, zero and unrestricted periods', () => {
  const limits = { daily_msat: '9007199254740993', weekly_msat: '0', per_payment_msat: '17' };
  const form = editLimits(limits);
  expect(form.daily_msat).toBe('9007199254740.993');
  expect(form.monthly_msat).toBe('');
  expect(parseLimits({ ...form, per_payment_msat: '0,017' })).toEqual(limits);
  for (const value of ['-1', '1.0001', '9223372036854775.808', '1e4']) {
    expect(parseLimits({ ...form, daily_msat: value })).toBeNull();
  }
  expect(parseLimits(editLimits())).toBeNull();
  expect(usedPercent('9007199254740993', '9007199254740994')).toBe(99);
  expect(usedPercent('5', '2')).toBe(100);
});
