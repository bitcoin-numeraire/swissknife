// Prepare the local files used by the Iconify and FlagIcon components.
import { cpSync, globSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { getIcons } from '@iconify/utils';

const require = createRequire(import.meta.url);
const root = dirname(fileURLToPath(import.meta.url));
const output = join(root, 'public/assets/dashboard-icons');
mkdirSync(output, { recursive: true });

const collections = ['carbon', 'eva', 'iconamoon', 'mdi', 'mingcute', 'solar'];
const references = new RegExp(`\\b(${collections.join('|')}):([a-z0-9-]+)`, 'g');
const names = new Map(collections.map((prefix) => [prefix, new Set()]));
for (const path of globSync('src/**/*.{ts,tsx}', { cwd: root })) {
  const source = readFileSync(join(root, path), 'utf8');
  for (const [, prefix, name] of source.matchAll(references)) {
    names.get(prefix).add(name);
  }
}

for (const [prefix, used] of names) {
  const collection = require(`@iconify-json/${prefix}/icons.json`);
  const icons = getIcons(collection, [...used].sort(), true);
  if (icons.not_found?.length) {
    throw new Error(`Unknown ${prefix} icons: ${icons.not_found.join(', ')}`);
  }
  writeFileSync(join(output, `${prefix}.json`), JSON.stringify(icons));
}

const flags = join(dirname(require.resolve('country-flag-icons')), '3x2');
const flagDir = join(output, 'flags');
mkdirSync(flagDir, { recursive: true });
for (const filename of globSync('*.svg', { cwd: flags })) {
  cpSync(join(flags, filename), join(flagDir, filename));
}
cpSync(join(flags, '../LICENSE'), join(flagDir, 'LICENSE'));
