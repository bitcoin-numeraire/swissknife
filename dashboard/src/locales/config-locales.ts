// ----------------------------------------------------------------------

export type LanguageValue = 'en' | 'fr';

export const fallbackLng = 'en';
export const languages = ['en', 'fr'];
export const defaultNS = 'common';
export const cookieName = 'i18next';

// ----------------------------------------------------------------------

export function i18nOptions(lng = fallbackLng, ns = defaultNS) {
  return {
    // debug: true,
    lng,
    fallbackLng,
    ns,
    defaultNS,
    fallbackNS: defaultNS,
    supportedLngs: languages,
  };
}

// ----------------------------------------------------------------------

export const changeLangMessages: Record<LanguageValue, { success: string; error: string; loading: string }> = {
  en: {
    success: 'Language has been changed!',
    error: 'Error changing language!',
    loading: 'Loading...',
  },
  fr: {
    success: 'La langue a été changée!',
    error: 'Erreur lors du changement de langue!',
    loading: 'Chargement...',
  },
};
