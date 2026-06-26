import { paths } from 'src/routes/paths';

export function normalizeRoutePath(path: string) {
  const [pathname] = path.split(/[?#]/);

  if (pathname === '/') return pathname;

  return pathname.replace(/\/+$/, '');
}

export function isSameRoutePath(currentPath: string, targetPath: string) {
  return normalizeRoutePath(currentPath) === normalizeRoutePath(targetPath);
}

export function isSetupRoutePath(path: string) {
  return [paths.onboarding.welcome, paths.auth.signUp].some((route) =>
    isSameRoutePath(path, route)
  );
}

export function isAuthRoutePath(path: string) {
  return [paths.auth.login, paths.auth.signUp, paths.onboarding.welcome].some((route) =>
    isSameRoutePath(path, route)
  );
}

export function getSafeReturnTo(returnTo: string | null, fallback = paths.wallet.root) {
  if (!returnTo || returnTo.startsWith('http://') || returnTo.startsWith('https://')) {
    return fallback;
  }

  if (isAuthRoutePath(returnTo)) {
    return fallback;
  }

  return returnTo.startsWith('/') ? returnTo : fallback;
}
