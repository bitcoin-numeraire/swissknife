import dayjs from 'dayjs';
import { it, expect } from 'vitest';

import { statementPeriod } from './wallet-statement-download';

it('exports every selected UTC calendar day across a clock change', () => {
  const range = statementPeriod(dayjs('2025-03-29'), dayjs('2025-03-30'));
  expect(range?.from.toISOString()).toBe('2025-03-29T00:00:00.000Z');
  expect(range?.until.toISOString()).toBe('2025-03-31T00:00:00.000Z');
  expect(statementPeriod(dayjs('2025-03-29'), dayjs('2025-03-28'))).toBeNull();
  expect(statementPeriod(null, dayjs('2025-03-30'))).toBeNull();
});
