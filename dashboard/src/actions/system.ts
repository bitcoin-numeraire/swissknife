import useSWR from 'swr';
import { useMemo } from 'react';

import { healthCheck } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useSystemHealth() {
  const result = useSWR(endpointKeys.system.health, () => healthCheck<true>());

  return useMemo(
    () => ({
      health: result.data?.data,
      healthLoading: result.isLoading,
      healthError: result.error,
      healthValidating: result.isValidating,
    }),
    [result]
  );
}
