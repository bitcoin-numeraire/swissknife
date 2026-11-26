import type { SpendingLimits } from 'src/lib/swissknife';

export const periods = ['daily_msat', 'weekly_msat', 'monthly_msat'] as const;
export const limitFields = [
  'per_payment_msat',
  'agent_hourly_msat',
  'agent_hourly_warning_msat',
  'approval_above_msat',
  ...periods,
] as const;
export type LimitField = (typeof limitFields)[number];
export type LimitsForm = Record<LimitField, string>;

// Form amounts are in sats. A template changes the editor until the operator saves.
export const policyTemplates = [
  {
    name: 'conservative',
    limits: {
      per_payment_msat: '10',
      agent_hourly_msat: '30',
      daily_msat: '100',
      weekly_msat: '500',
      monthly_msat: '2000',
    },
  },
  {
    name: 'moderate',
    limits: {
      per_payment_msat: '50',
      agent_hourly_msat: '150',
      daily_msat: '500',
      weekly_msat: '2500',
      monthly_msat: '10000',
    },
  },
  {
    name: 'generous',
    limits: {
      per_payment_msat: '100',
      agent_hourly_msat: '300',
      daily_msat: '1000',
      weekly_msat: '5000',
      monthly_msat: '20000',
    },
  },
] as const satisfies ReadonlyArray<{
  name: string;
  limits: Omit<LimitsForm, 'approval_above_msat' | 'agent_hourly_warning_msat'>;
}>;

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
