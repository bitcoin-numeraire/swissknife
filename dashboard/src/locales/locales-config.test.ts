import { createInstance } from 'i18next';
import { vi, it, expect, afterEach } from 'vitest';

import enCommon from './langs/en/common.json';
import frCommon from './langs/fr/common.json';
import enWelcome from './langs/en/welcome.json';
import frWelcome from './langs/fr/welcome.json';

afterEach(() => {
  vi.unstubAllEnvs();
  vi.resetModules();
});

it.each([
  { configured: 'Numeraire', expected: 'Numeraire' },
  { configured: 'SwissKnife', expected: 'SwissKnife' },
  { configured: 'Acme & Co', expected: 'Acme & Co' },
  { configured: undefined, expected: 'Numeraire SwissKnife' },
  { configured: '  ', expected: 'Numeraire SwissKnife' },
])(
  'uses the deployment name $expected throughout the interface',
  async ({ configured, expected }) => {
    vi.stubEnv('NEXT_PUBLIC_APPNAME', configured);
    const { i18nOptions } = await import('./locales-config');
    const { default: manifest } = await import('../app/manifest');
    const i18n = createInstance();
    await i18n.init({
      ...i18nOptions(),
      ns: ['common', 'welcome'],
      resources: {
        en: { common: enCommon, welcome: enWelcome },
        fr: { common: frCommon, welcome: frWelcome },
      },
    });

    expect(i18n.t('send_money.request_type_internal')).toBe(`${expected} internal`);
    expect(i18n.t('settings_view.account_id_description')).toBe(
      `Stable identifier for your ${expected} account.`
    );
    expect(i18n.t('preview.eyebrow', { ns: 'welcome' })).toBe(`${expected} setup`);

    await i18n.changeLanguage('fr');
    expect(i18n.t('send_money.request_type_internal')).toBe(`Interne ${expected}`);
    expect(i18n.t('settings_view.account_id_description')).toBe(
      `Identifiant stable de ton compte ${expected}.`
    );
    expect(i18n.t('preview.eyebrow', { ns: 'welcome' })).toBe(`Configuration ${expected}`);
    expect(manifest()).toMatchObject({ name: expected, short_name: expected });
  }
);

function messages(value: Record<string, unknown>, parent = ''): Record<string, string[]> {
  return Object.fromEntries(
    Object.entries(value).flatMap(([key, text]) => {
      const path = parent ? `${parent}.${key}` : key;
      if (typeof text === 'string') {
        expect(text.trim(), path).not.toBe('');
        return [[path, [...text.matchAll(/\{\{[^{}]+\}\}/g)].map(([match]) => match).sort()]];
      }
      return Object.entries(messages(text as Record<string, unknown>, path));
    })
  );
}

it('ships both namespaces and all interpolation tokens in every supported language', async () => {
  const { languages } = await import('./locales-config');
  for (const lng of languages) {
    for (const [ns, english] of [
      ['common', enCommon],
      ['welcome', enWelcome],
    ] as const) {
      const localized = await import(`./langs/${lng}/${ns}.json`);
      expect(messages(localized.default), `${lng}/${ns}`).toEqual(messages(english));
    }
  }
});
