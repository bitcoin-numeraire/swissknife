import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { preferredLanguage, translate, translations } from '../ui/languages.mjs';

test('selects the first supported system preference, including regional variants', () => {
  assert.equal(preferredLanguage(['rm-CH', 'fr-CH', 'de-CH']), 'fr');
  assert.equal(preferredLanguage(['PT-BR']), 'pt');
  assert.equal(preferredLanguage(['en-GB', 'it']), 'en');
  assert.equal(preferredLanguage(['ja-JP']), 'en');
  assert.equal(preferredLanguage([]), 'en');
});

test('translates every launcher label and address validation message', () => {
  const html = readFileSync(new URL('../ui/index.html', import.meta.url), 'utf8');
  const labels = Array.from(html.matchAll(/ data-i18n>([^<]+)</g), ([, label]) => label);
  assert.ok(labels.includes('Open wallet'));
  const source = readFileSync(new URL('../src/main.rs', import.meta.url), 'utf8');
  const validation = source.slice(source.indexOf('fn origin('), source.indexOf('fn can_navigate('));
  const errors = Array.from(validation.matchAll(/"([^"\n]+\.)"/g), ([, message]) => message);
  assert.ok(errors.includes('Enter a complete instance URL.'));
  for (const message of [...labels, ...errors, 'Open the connection from the desktop launcher.']) {
    assert.equal(translate('en', message), message);
    for (const locale of ['de', 'es', 'fr', 'it', 'pt']) {
      assert.ok(translations[message]?.[locale]?.trim(), locale + ' missing: ' + message);
    }
  }
  assert.equal(translate('fr', 'Webview runtime unavailable'), 'Webview runtime unavailable');
});
