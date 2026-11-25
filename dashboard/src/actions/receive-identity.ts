import useSWR from 'swr';

import { CONFIG } from 'src/global-config';
import { receiveIdentity } from 'src/lib/swissknife';

const unset = { domain: null, lnurl_base_url: null };

export function useReceiveIdentity() {
  const { data } = useSWR('systemReceiveIdentity', async () => {
    const result = await receiveIdentity({ throwOnError: false });
    if (result.response?.status === 404 && CONFIG.domain) {
      return {
        domain: CONFIG.domain,
        lnurl_base_url: 'https://' + CONFIG.domain + '/.well-known/lnurlp/',
      };
    }
    if (!result.response?.ok) throw new Error('Unable to read the public receive identity');
    return result.data;
  });
  return data ?? unset;
}
