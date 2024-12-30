import type { IBreezLSP, IBreezNodeInfo } from 'src/types/breez-node';

import useSWR from 'swr';
import { useMemo } from 'react';

import { lspInfo, listLsps, nodeInfo } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

export function useGetNodeInfo() {
  const fetcher = async () => {
    const { data, error } = await nodeInfo();
    if (error) {
      throw Error(error.reason);
    }

    return data as IBreezNodeInfo;
  };

  const { data, isLoading, error, isValidating } = useSWR(
    endpointKeys.lightning.node.info,
    fetcher
  );

  return useMemo(
    () => ({
      nodeInfo: data,
      nodeInfoLoading: isLoading,
      nodeInfoError: error,
      nodeInfoValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

export function useGetCurrentLSP() {
  const fetcher = async () => {
    const { data, error } = await lspInfo();
    if (error) {
      throw Error(error.reason);
    }

    return data as IBreezLSP;
  };

  const { data, isLoading, error, isValidating } = useSWR(
    endpointKeys.lightning.node.lspInfo,
    fetcher
  );

  return useMemo(
    () => ({
      currentLSP: data,
      currentLSPLoading: isLoading,
      currentLSPError: error,
      currentLSPValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

export function useGetLSPs() {
  const fetcher = async () => {
    const { data, error } = await listLsps();
    if (error) {
      throw Error(error.reason);
    }

    return data as IBreezLSP[];
  };

  const { data, isLoading, error, isValidating } = useSWR(
    endpointKeys.lightning.node.lsps,
    fetcher
  );

  return useMemo(
    () => ({
      lsps: data as IBreezLSP[],
      lspsLoading: isLoading,
      lspsError: error,
      lspsValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}
