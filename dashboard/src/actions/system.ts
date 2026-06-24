import type { HealthCheck } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useMemo } from 'react';

import { healthCheck, HealthStatus } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

const HEALTH_CHECK_TIMEOUT_MS = 2500;

const HEALTH_CHECK_FALLBACK: HealthCheck = {
  is_healthy: false,
  database: HealthStatus.MAINTENANCE,
  ln_provider: HealthStatus.UNAVAILABLE,
};

type HealthCheckState = {
  health: HealthCheck;
  degraded?: boolean;
  degradedReason?: 'timeout' | 'unavailable';
};

export function useSystemHealth() {
  const fetcher = async (): Promise<HealthCheckState> => {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), HEALTH_CHECK_TIMEOUT_MS);

    try {
      const { data, error } = await healthCheck({
        signal: controller.signal,
      });

      if (data) {
        return { health: data };
      }

      if (error) {
        return {
          health: error,
          degraded: true,
          degradedReason: 'unavailable',
        };
      }

      return {
        health: HEALTH_CHECK_FALLBACK,
        degraded: true,
        degradedReason: 'unavailable',
      };
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        return {
          health: HEALTH_CHECK_FALLBACK,
          degraded: true,
          degradedReason: 'timeout',
        };
      }

      throw error;
    } finally {
      window.clearTimeout(timeout);
    }
  };

  const result = useSWR(endpointKeys.system.health, fetcher);

  return useMemo(
    () => ({
      health: result.data?.health,
      healthDegraded: result.data?.degraded,
      healthDegradedReason: result.data?.degradedReason,
      healthLoading: result.isLoading,
      healthError: result.error,
      healthValidating: result.isValidating,
    }),
    [result]
  );
}
