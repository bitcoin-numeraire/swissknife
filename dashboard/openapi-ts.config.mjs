/** @type {import('@hey-api/openapi-ts').UserConfig} */
export default {
  input: 'src/lib/openapi.json',
  output: {
    path: 'src/lib/swissknife',
    postProcess: ['prettier'],
  },
  plugins: [
    '@hey-api/client-fetch',
    {
      name: '@hey-api/typescript',
      enums: 'javascript',
    },
    {
      name: '@hey-api/transformers',
      dates: true,
      bigInt: false,
    },
    {
      name: '@hey-api/sdk',
      transformer: true,
    },
    'zod',
  ],
};
