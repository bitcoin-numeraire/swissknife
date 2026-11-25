import type { SpendingLimits } from 'src/lib/swissknife';

export const periods = ['daily_msat', 'weekly_msat', 'monthly_msat'] as const;
export const limitFields = ['per_payment_msat', 'agent_hourly_msat', ...periods] as const;
export type LimitField = (typeof limitFields)[number];
export type LimitsForm = Record<LimitField, string>;

export function satsFromMsat(msat: string): string {
  const value = BigInt(msat);
  const fraction = (value % BigInt(1000)).toString().padStart(3, '0').replace(/0+$/, '');
  return (value / BigInt(1000)).toString() + (fraction ? '.' + fraction : '');
}

export function editLimits(limits?: SpendingLimits): LimitsForm {
  return Object.fromEntries(
    limitFields.map((field) => [field, limits?.[field] != null ? satsFromMsat(limits[field]) : ''])
  ) as LimitsForm;
}

export function parseLimits(form: LimitsForm): SpendingLimits | null {
  const limits: SpendingLimits = {};
  for (const field of limitFields) {
    const text = form[field].trim();
    if (!text) continue;
    if (!/^\d{1,16}([.,]\d{1,3})?$/.test(text)) return null;
    const [whole, fraction = ''] = text.split(/[.,]/);
    const value = BigInt(whole) * BigInt(1000) + BigInt(fraction.padEnd(3, '0'));
    if (value > BigInt('9223372036854775807')) return null;
    limits[field] = value.toString();
  }
  return Object.keys(limits).length ? limits : null;
}

export function usedPercent(usage: string, cap: string) {
  const maximum = BigInt(cap);
  if (maximum === BigInt(0)) return 100;
  const percent = (BigInt(usage) * BigInt(100)) / maximum;
  return Number(percent > BigInt(100) ? BigInt(100) : percent);
}
