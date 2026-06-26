import { jwtDecode } from 'jwt-decode';

import { paths } from 'src/routes/paths';

import { client } from 'src/lib/swissknife/client.gen';

import { JWT_STORAGE_KEY } from './constant';

// ----------------------------------------------------------------------

let requestInterceptorId: number | null = null;
let errorInterceptorId: number | null = null;

function normalizePath(path: string) {
  if (path === '/') return path;

  return path.replace(/\/+$/, '');
}

function isSetupPath(pathname: string) {
  const currentPath = normalizePath(pathname);

  return [paths.onboarding.welcome, paths.auth.signUp].some(
    (path) => normalizePath(path) === currentPath
  );
}

export function clearSession() {
  sessionStorage.removeItem(JWT_STORAGE_KEY);

  if (requestInterceptorId !== null) {
    client.interceptors.request.eject(requestInterceptorId);
    requestInterceptorId = null;
  }

  if (errorInterceptorId !== null) {
    client.interceptors.error.eject(errorInterceptorId);
    errorInterceptorId = null;
  }
}

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
    clearSession();
    sessionStorage.setItem(JWT_STORAGE_KEY, accessToken);

    requestInterceptorId = client.interceptors.request.use((request, _) => {
      request.headers.set('Authorization', `Bearer ${accessToken}`);
      return request;
    });

    errorInterceptorId = client.interceptors.error.use((error, response) => {
      if (response?.status === 401) {
        clearSession();

        if (
          normalizePath(window.location.pathname) !== normalizePath(paths.auth.login) &&
          !isSetupPath(window.location.pathname)
        ) {
          window.location.replace(paths.auth.login);
        }
      }

      return Promise.reject(error);
    });
  } catch (error) {
    console.error('Error during set session:', error);
    throw error;
  }
}
