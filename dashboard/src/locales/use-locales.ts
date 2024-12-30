'use client';

import dayjs from 'dayjs';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';

import { useRouter } from 'src/routes/hooks';

import { allLangs } from './all-langs';
import { fallbackLng } from './locales-config';

import type { LanguageValue } from './locales-config';

// ----------------------------------------------------------------------

export function useTranslate(ns?: string) {
  const router = useRouter();

  const { t, i18n } = useTranslation(ns);

  const fallback = allLangs.filter((lang) => lang.value === fallbackLng)[0];

  const currentLang = allLangs.find((lang) => lang.value === i18n.resolvedLanguage);

  const onChangeLang = useCallback(
    async (newLang: LanguageValue) => {
      try {
        await i18n.changeLanguage(newLang);

        if (currentLang) {
          dayjs.locale(currentLang.adapterLocale);
        }

        router.refresh();
      } catch (error) {
        console.error(error);
      }
    },
    [currentLang, i18n, router]
  );

  return {
    t,
    i18n,
    onChangeLang,
    currentLang: currentLang ?? fallback,
  };
}
