// ----------------------------------------------------------------------

export const fallbackLng = 'en';
export const languages = ['en', 'fr'];
export const defaultNS = 'common';
export const cookieName = 'i18next';

export type LanguageValue = (typeof languages)[number];

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
