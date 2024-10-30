/** @type {import('@hey-api/openapi-ts').UserConfig} */
export default {
  client: '@hey-api/client-fetch',
  input: 'src/lib/openapi.json',
  output: {
    format: 'prettier',
    lint: 'eslint',
    path: 'src/lib/swissknife',
  },
  types: {
    dates: 'types+transform',
    enums: 'javascript',
  },
  schemas: {
    type: 'form',
  },
};
