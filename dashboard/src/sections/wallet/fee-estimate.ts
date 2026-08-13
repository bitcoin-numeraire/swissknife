import type { PaymentFeeEstimate } from 'src/lib/swissknife';

export function getFeeEstimateState(
  estimate: PaymentFeeEstimate | undefined,
  availableMsat: number | undefined
) {
  const displayFeeMsat = estimate?.estimated_fee_msat ?? estimate?.maximum_fee_msat;
  const maximumOnly = estimate != null && estimate.estimated_fee_msat == null;
  const hasDistinctMaximum =
    estimate?.estimated_fee_msat != null && estimate.maximum_fee_msat > estimate.estimated_fee_msat;
  const exceedsAvailable =
    estimate != null &&
    availableMsat != null &&
    estimate.maximum_total_msat > Math.max(availableMsat, 0);

  return {
    displayFeeMsat,
    maximumOnly,
    hasDistinctMaximum,
    exceedsAvailable,
  };
}
