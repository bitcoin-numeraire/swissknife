import { paths } from 'src/routes/paths';

import packageJson from '../package.json';
import { client } from './lib/swissknife/client.gen';

// ----------------------------------------------------------------------

export type ConfigValue = {
  appName: string;
  appVersion: string;
  serverUrl: string;
  assetsDir: string;
  isStaticExport: boolean;
  deploymentMode: DeploymentMode;
  domain: string;
  mempoolSpace: string;
  auth: {
    method: AuthMethod;
    skip: boolean;
    redirectPath: string;
  };
  auth0: { clientId: string; domain: string; callbackUrl: string; audience: string };
  mockOAuth2: { tokenUrl: string; clientSecret: string };
  supabase: { url: string; key: string };
};

export type AuthMethod = 'jwt' | 'supabase' | 'auth0' | 'mock-oauth2';
export type DeploymentMode = 'server' | 'self-hosted' | 'merchant' | 'desktop' | 'agent';

// ----------------------------------------------------------------------

export const CONFIG: ConfigValue = {
  appName: process.env.NEXT_PUBLIC_APPNAME ?? 'Numeraire SwissKnife',
  appVersion: packageJson.version,
  serverUrl: process.env.NEXT_PUBLIC_SERVER_URL ?? '',
  assetsDir: process.env.NEXT_PUBLIC_ASSETS_DIR ?? '',
  isStaticExport: JSON.parse(`${process.env.BUILD_STATIC_EXPORT}`),
  deploymentMode: (process.env.NEXT_PUBLIC_DEPLOYMENT_MODE as DeploymentMode) ?? 'server',
  domain: process.env.NEXT_PUBLIC_DOMAIN ?? 'numeraire.tech',
  mempoolSpace: process.env.NEXT_PUBLIC_MEMPOOL_SPACE_URL ?? 'https://mempool.space/api/v1',
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
   * Mock OAuth2
   */
  mockOAuth2: {
    tokenUrl:
      process.env.NEXT_PUBLIC_MOCK_OAUTH2_TOKEN_URL ?? 'http://127.0.0.1:8090/default/token',
    clientSecret: process.env.NEXT_PUBLIC_MOCK_OAUTH2_CLIENT_SECRET ?? 'dev-secret',
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
  baseUrl: CONFIG.serverUrl,
  throwOnError: true,
});
