import type { AuthMethod } from 'src/global-config';

import { CONFIG } from 'src/global-config';

export function authUsesLocalTokenSession(method: AuthMethod = CONFIG.auth.method) {
  return method === 'jwt' || method === 'mock-oauth2';
}

export function authRequiresLocalSignUp(method: AuthMethod = CONFIG.auth.method) {
  return method === 'jwt';
}
