import { paths } from 'src/routes/paths';

import packageJson from '../package.json';
import { client } from './lib/swissknife';

// ----------------------------------------------------------------------

export type ConfigValue = {
  isStaticExport: boolean;
  site: {
    name: string;
    serverUrl: string;
    assetURL: string;
    basePath: string;
    version: string;
    domain: string;
    mempoolSpace: string;
  };
  auth: {
    method: 'jwt' | 'supabase' | 'auth0';
    skip: boolean;
    redirectPath: string;
  };
  auth0: { clientId: string; domain: string; callbackUrl: string; audience: string };
  supabase: { url: string; key: string };
};

export type AuthMethod = 'jwt' | 'supabase' | 'auth0';

// ----------------------------------------------------------------------

export const CONFIG: ConfigValue = {
  site: {
    name: process.env.NEXT_PUBLIC_SITENAME ?? 'Numeraire SwissKnife',
    serverUrl: process.env.NEXT_PUBLIC_SERVER_URL ?? '',
    assetURL: process.env.NEXT_PUBLIC_ASSET_URL ?? '',
    basePath: process.env.NEXT_PUBLIC_BASE_PATH ?? '',
    domain: process.env.NEXT_PUBLIC_DOMAIN ?? 'numeraire.tech',
    mempoolSpace: process.env.NEXT_PUBLIC_MEMPOOL_SPACE_URL ?? 'https://mempool.space/api/v1',
    version: packageJson.version,
  },
  isStaticExport: JSON.parse(`${process.env.BUILD_STATIC_EXPORT}`),
  /**
   * Auth
   * @method {AuthMethod}
   */
  auth: {
    method: (process.env.NEXT_PUBLIC_AUTH_METHOD as AuthMethod) ?? 'jwt',
    skip: false,
    redirectPath: paths.wallet.root,
  },
  /**
   * Auth0
   */
  auth0: {
    clientId: process.env.NEXT_PUBLIC_AUTH0_CLIENT_ID ?? '',
    domain: process.env.NEXT_PUBLIC_AUTH0_DOMAIN ?? '',
    callbackUrl: process.env.NEXT_PUBLIC_AUTH0_CALLBACK_URL ?? '',
    audience: process.env.NEXT_PUBLIC_AUTH0_AUDIENCE ?? 'https://swissknife.numeraire.tech/api/v1',
  },
  /**
   * Supabase
   */
  supabase: {
    url: process.env.NEXT_PUBLIC_SUPABASE_URL ?? '',
    key: process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY ?? '',
  },
};

client.setConfig({
  baseUrl: CONFIG.site.serverUrl,
  throwOnError: true,
});
