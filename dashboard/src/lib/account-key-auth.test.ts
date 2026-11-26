import { it, expect } from 'vitest';

import { createClient } from './swissknife/client';
import { readinessCheck, listAccountWallets } from './swissknife';

it('uses the selected account credential for wallet discovery and omits it on readiness checks', async () => {
  const requests: Request[] = [];
  const client = createClient({
    baseUrl: 'https://swissknife.example',
    auth: (scheme) => (scheme.name === 'Api-Key' ? 'account-key' : undefined),
    fetch: async (input) => {
      const request = new Request(input);
      requests.push(request);
      return new URL(request.url).pathname === '/v1/system/ready'
        ? new Response(null, { status: 204 })
        : Response.json([]);
    },
  });

  await listAccountWallets({ client });
  await readinessCheck({ client });

  const [wallets, readiness] = requests;
  expect(wallets.headers.get('Api-Key')).toBe('account-key');
  expect(wallets.headers.has('Authorization')).toBe(false);
  expect(wallets.headers.has('Wallet-Key')).toBe(false);
  expect(readiness.headers.has('Api-Key')).toBe(false);
});
