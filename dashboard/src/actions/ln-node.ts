import type { IBreezLSP, IBreezNodeInfo } from 'src/types/breez-node';

import useSWR from 'swr';
import { useMemo } from 'react';

import { lspInfo, listLsps, nodeInfo } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

// ----------------------------------------------------------------------

type IGetNodeInfo = {
  nodeInfo?: IBreezNodeInfo;
  nodeInfoLoading: boolean;
  nodeInfoError: any;
  nodeInfoValidating: boolean;
};

export function useGetNodeInfo(): IGetNodeInfo {
  const fetcher = async () => {
    const { data, error } = await nodeInfo();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, isLoading, error, isValidating } = useSWR(endpointKeys.lightning.node.info, fetcher);

  return useMemo(
    () => ({
      nodeInfo: data as IBreezNodeInfo,
      nodeInfoLoading: isLoading,
      nodeInfoError: error,
      nodeInfoValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

type IGetCurrentLSP = {
  currentLSP?: IBreezLSP;
  currentLSPLoading: boolean;
  currentLSPError: any;
  currentLSPValidating: boolean;
};

export function useGetCurrentLSP(): IGetCurrentLSP {
  const fetcher = async () => {
    const { data, error } = await lspInfo();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, isLoading, error, isValidating } = useSWR(endpointKeys.lightning.node.lspInfo, fetcher);

  return useMemo(
    () => ({
      currentLSP: data as IBreezLSP,
      currentLSPLoading: isLoading,
      currentLSPError: error,
      currentLSPValidating: isValidating,
    }),
    [data, error, isLoading, isValidating]
  );
}

type IGetLSPs = {
  lsps?: IBreezLSP[];
  lspsLoading: boolean;
  lspsError: any;
  lspsValidating: boolean;
};

export function useGetLSPs(): IGetLSPs {
  const fetcher = async () => {
    const { data, error } = await listLsps();
    if (error) {
      throw Error(error.reason);
    }

    return data;
  };

  const { data, isLoading, error, isValidating } = useSWR(endpointKeys.lightning.node.lsps, fetcher);

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
