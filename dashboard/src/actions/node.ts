import useSWR from 'swr';

import { readNodeOverview } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

export function useNodeOverview() {
  return useSWR(endpointKeys.lightning.node.info, async () => {
    const { data } = await readNodeOverview({ throwOnError: true });
    return data;
  });
}
