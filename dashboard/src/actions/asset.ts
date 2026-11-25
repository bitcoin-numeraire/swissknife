import useSWR from 'swr';

import { listAssets } from 'src/lib/swissknife';

import { endpointKeys } from './keys';

export function useListAssets() {
  return useSWR(endpointKeys.assets.list, async () => (await listAssets<true>()).data);
}
