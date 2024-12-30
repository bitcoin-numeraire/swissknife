import { defaultPlugins } from '@hey-api/openapi-ts';

/** @type {import('@hey-api/openapi-ts').UserConfig} */
export default {
  client: '@hey-api/client-fetch',
  experimentalParser: true,
  input: 'src/lib/openapi.json',
  output: {
    format: 'prettier',
    lint: 'eslint',
    path: 'src/lib/swissknife',
  },
  plugins: [
    ...defaultPlugins,
    {
      name: '@hey-api/typescript',
      enums: 'javascript',
    },
    {
      dates: true,
      name: '@hey-api/transformers',
    },
    'zod',
    {
      name: '@hey-api/sdk',
    },
  ],
};
