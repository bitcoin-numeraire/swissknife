import { jwtDecode } from 'jwt-decode';

import { paths } from 'src/routes/paths';

import { client } from 'src/lib/swissknife';

import { JWT_STORAGE_KEY } from './constant';

// ----------------------------------------------------------------------

export function isValidToken(accessToken: string) {
  if (!accessToken) {
    return false;
  }

  try {
    const decoded = jwtDecode(accessToken);

    if (!decoded || !decoded.exp) {
      return false;
    }

    const currentTime = Date.now() / 1000;

    return decoded.exp > currentTime;
  } catch (error) {
    console.error('Error during token validation:', error);
    return false;
  }
}

// ----------------------------------------------------------------------

export async function setSession(accessToken: string) {
  try {
    sessionStorage.setItem(JWT_STORAGE_KEY, accessToken);

    client.interceptors.request.use((request, _) => {
      request.headers.set('Authorization', `Bearer ${accessToken}`);
      return request;
    });

    client.interceptors.error.use((error, response) => {
      if (response.status === 401) {
        sessionStorage.removeItem(JWT_STORAGE_KEY);
        window.location.href = paths.auth.jwt.signIn;
      }

      return Promise.reject(error);
    });
  } catch (error) {
    console.error('Error during set session:', error);
    throw error;
  }
}
