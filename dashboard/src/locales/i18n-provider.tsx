'use client';

import i18next from 'i18next';
import { useMemo } from 'react';
import { getStorage } from 'minimal-shared/utils';
import resourcesToBackend from 'i18next-resources-to-backend';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next, I18nextProvider as Provider } from 'react-i18next';

import { CONFIG } from 'src/global-config';

import { i18nOptions, fallbackLng } from './locales-config';

import type { LanguageValue } from './locales-config';

// ----------------------------------------------------------------------

let lng;

/**
 * [1] localStorage
 * Auto detection:
 * const lng = getStorage('i18nextLng')
 */
if (CONFIG.isStaticExport) {
  lng = getStorage('i18nextLng', fallbackLng) as string;
}

const init = CONFIG.isStaticExport
  ? { ...i18nOptions(lng), detection: { caches: ['localStorage'] } }
  : { ...i18nOptions(), detection: { caches: ['cookie'] } };

i18next
  .use(LanguageDetector)
  .use(initReactI18next)
  .use(resourcesToBackend((lang: string, ns: string) => import(`./langs/${lang}/${ns}.json`)))
  .init(init);

// ----------------------------------------------------------------------

type Props = {
  lang?: LanguageValue | undefined;
  children: React.ReactNode;
};

export function I18nProvider({ lang, children }: Props) {
  useMemo(() => {
    if (lang) {
      i18next.changeLanguage(lang);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <Provider i18n={i18next}>{children}</Provider>;
}
