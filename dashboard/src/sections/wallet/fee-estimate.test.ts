import type { PaymentFeeEstimate } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';

import { getFeeEstimateState } from './fee-estimate';

const estimate: PaymentFeeEstimate = {
  ledger: 'Lightning',
  amount_msat: 100_000,
  estimated_fee_msat: 900,
  maximum_fee_msat: 5_000,
  estimated_total_msat: 100_900,
  maximum_total_msat: 105_000,
};

describe('getFeeEstimateState', () => {
  it('shows the route estimate and a distinct enforced maximum', () => {
    expect(getFeeEstimateState(estimate, 200_000)).toEqual({
      displayFeeMsat: 900,
      maximumOnly: false,
      hasDistinctMaximum: true,
      exceedsAvailable: false,
    });
  });

  it('shows only the maximum when route estimation is unavailable', () => {
    expect(
      getFeeEstimateState(
        {
          ...estimate,
          estimated_fee_msat: null,
          estimated_total_msat: null,
        },
        200_000
      )
    ).toMatchObject({
      displayFeeMsat: 5_000,
      maximumOnly: true,
      hasDistinctMaximum: false,
    });
  });

  it('uses the maximum total for balance admission', () => {
    expect(getFeeEstimateState(estimate, 104_999).exceedsAvailable).toBe(true);
    expect(getFeeEstimateState(estimate, 105_000).exceedsAvailable).toBe(false);
  });
});
