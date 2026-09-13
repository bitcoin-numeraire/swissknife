import type { PaymentFeeEstimate } from 'src/lib/swissknife';

import { it, expect, describe } from 'vitest';
import { act, renderHook } from '@testing-library/react';

import { useFeeEstimate } from './use-fee-estimate';

const quote: PaymentFeeEstimate = {
  ledger: 'Lightning',
  amount_msat: 100_000,
  estimated_fee_msat: 900,
  maximum_fee_msat: 5_000,
  estimated_total_msat: 100_900,
  maximum_total_msat: 105_000,
};

describe('fee estimate requests', () => {
  it('discards an in-flight quote after the payment request changes', async () => {
    const { result, rerender } = renderHook(({ key }) => useFeeEstimate(key), {
      initialProps: { key: 'wallet-1:recipient-1:100000' },
    });
    let resolve!: (value: PaymentFeeEstimate) => void;
    const response = new Promise<PaymentFeeEstimate>((done) => {
      resolve = done;
    });
    let pending!: Promise<void>;
    act(() => {
      pending = result.current.estimate(() => response);
    });
    rerender({ key: 'wallet-2:recipient-2:200000' });
    await act(async () => {
      resolve(quote);
      await pending;
    });
    expect(result.current.feeEstimate).toBeUndefined();
    expect(result.current.isEstimatingFee).toBe(false);
  });

  it('keeps a newer quote when an older request completes last', async () => {
    const { result } = renderHook(() => useFeeEstimate('request'));
    let resolve!: (value: PaymentFeeEstimate) => void;
    const response = new Promise<PaymentFeeEstimate>((done) => {
      resolve = done;
    });
    let pending!: Promise<void>;
    act(() => {
      pending = result.current.estimate(() => response);
    });
    const newer = { ...quote, estimated_fee_msat: 1_000 };
    await act(async () => {
      await result.current.estimate(async () => newer);
    });
    await act(async () => {
      resolve(quote);
      await pending;
    });
    expect(result.current.feeEstimate).toEqual(newer);
  });

  it('suppresses errors from a cancelled request', async () => {
    const { result } = renderHook(() => useFeeEstimate('request'));
    let reject!: (error: Error) => void;
    const response = new Promise<PaymentFeeEstimate>((_, fail) => {
      reject = fail;
    });
    let pending!: Promise<void>;
    act(() => {
      pending = result.current.estimate(() => response);
    });
    act(() => result.current.reset());
    await act(async () => {
      reject(new Error('old request'));
      await pending;
    });
    expect(result.current.isEstimatingFee).toBe(false);
  });
});
