import type { PaymentFeeEstimate } from 'src/lib/swissknife';

import { useRef, useState, useEffect, useCallback } from 'react';

export function useFeeEstimate(requestKey: string) {
  const pending = useRef<AbortController | null>(null);
  const [result, setResult] = useState<{ key: string; value: PaymentFeeEstimate }>();
  const [loadingKey, setLoadingKey] = useState<string>();

  const reset = useCallback(() => {
    pending.current?.abort();
    pending.current = null;
    setResult(undefined);
    setLoadingKey(undefined);
  }, []);

  useEffect(() => {
    reset();
    return () => pending.current?.abort();
  }, [requestKey, reset]);

  const estimate = async (request: (signal: AbortSignal) => Promise<PaymentFeeEstimate>) => {
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    setLoadingKey(requestKey);
    try {
      const value = await request(controller.signal);
      if (!controller.signal.aborted) setResult({ key: requestKey, value });
    } catch (error) {
      if (!controller.signal.aborted) throw error;
    } finally {
      if (pending.current === controller) {
        pending.current = null;
        setLoadingKey(undefined);
      }
    }
  };

  return {
    feeEstimate: result?.key === requestKey ? result.value : undefined,
    isEstimatingFee: loadingKey === requestKey,
    estimate,
    reset,
  };
}
