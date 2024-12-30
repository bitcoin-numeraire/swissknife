import { cookies as getCookies } from 'next/headers';

import { cookieName, fallbackLng } from './locales-config';

import type { LanguageValue } from './locales-config';

// ----------------------------------------------------------------------

/**
 * [1] with url:
 * https://nextjs.org/docs/pages/building-your-application/routing/internationalization
 *
 * Use i18next with app folder and without locale in url:
 * https://github.com/i18next/next-app-dir-i18next-example/issues/12#issuecomment-1500917570
 */

export async function detectLanguage() {
  const cookies = getCookies();

  const language = cookies.get(cookieName)?.value ?? fallbackLng;

  return language as LanguageValue;
}
