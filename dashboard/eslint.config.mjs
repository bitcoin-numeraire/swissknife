import globals from 'globals';
import eslintJs from '@eslint/js';
import eslintTs from 'typescript-eslint';
import pluginReact from 'eslint-plugin-react';
import { globalIgnores } from 'eslint/config';
import pluginImport from 'eslint-plugin-import';
import pluginReactHooks from 'eslint-plugin-react-hooks';
import pluginPerfectionist from 'eslint-plugin-perfectionist';
import pluginUnusedImports from 'eslint-plugin-unused-imports';

// ----------------------------------------------------------------------

/**
 * @config reactConfig
 * Packages:
 * - 'eslint-plugin-react'
 * - 'eslint-plugin-react-hooks'
 * - '@typescript-eslint/eslint-plugin'
 * Core rules for React, React Hooks, and general JavaScript/TypeScript.
 */
const reactConfig = {
  plugins: {
    'react-hooks': pluginReactHooks,
  },
  settings: {
    react: { version: 'detect' },
  },
  rules: {
    ...pluginReactHooks.configs.recommended.rules,
    'func-names': 1,
    'no-bitwise': 2,
    'object-shorthand': 1,
    'no-useless-rename': 1,
    'default-case-last': 2,
    'consistent-return': 2,
    'no-constant-condition': 1,
    'default-case': [2, { commentPattern: '^no default$' }],
    'lines-around-directive': [2, { before: 'always', after: 'always' }],
    'arrow-body-style': [2, 'as-needed', { requireReturnForObjectLiteral: false }],
    // --- react ---
    'react/jsx-key': 0,
    'react/prop-types': 0,
    'react/display-name': 0,
    'react/no-children-prop': 0,
    'react/jsx-boolean-value': 2,
    'react/self-closing-comp': 2,
    'react/react-in-jsx-scope': 0,
    'react/jsx-no-useless-fragment': [1, { allowExpressions: true }],
    'react/jsx-curly-brace-presence': [2, { props: 'never', children: 'never' }],
    // --- react hooks ---
    'react-hooks/refs': 0,
    'react-hooks/immutability': 0,
    'react-hooks/set-state-in-effect': 0,
    'react-hooks/incompatible-library': 0,
    'react-hooks/preserve-manual-memoization': 0,
    // --- typescript ---
    '@typescript-eslint/no-shadow': 2,
    '@typescript-eslint/no-explicit-any': 0,
    '@typescript-eslint/no-empty-object-type': 0,
    '@typescript-eslint/consistent-type-imports': 1,
  },
};

/**
 * @config importConfig
 * Package: 'eslint-plugin-import'
 * Management of module resolution and import/export rules.
 */
const importConfig = {
  plugins: {
    import: pluginImport,
  },
  settings: {
    // https://www.npmjs.com/package/eslint-import-resolver-typescript
    ...pluginImport.configs.typescript.settings,
    'import/resolver': {
      ...(pluginImport.configs.typescript.settings?.['import/resolver'] ?? {}),
      typescript: {
        project: './tsconfig.json',
      },
    },
  },
  rules: {
    ...pluginImport.configs.recommended.rules,
    'import/named': 0,
    'import/export': 0,
    'import/default': 0,
    'import/namespace': 0,
    'import/no-named-as-default': 0,
    'import/newline-after-import': 2,
    'import/no-named-as-default-member': 0,
    'import/no-cycle': [
      0, // Disabled by default for performance; enable manually to check for circular dependencies
      { ignoreExternal: true, disableScc: true },
    ],
  },
};

/**
 * @config unusedImportsConfig
 * Package: 'eslint-plugin-unused-imports'
 * Automates the detection and cleanup of unused modules and variables.
 */
const unusedImportsConfig = {
  plugins: {
    'unused-imports': pluginUnusedImports,
  },
  rules: {
    'no-unused-vars': 0, // Handled by 'eslint-plugin-unused-imports'
    '@typescript-eslint/no-unused-vars': 0, // Handled by 'eslint-plugin-unused-imports'
    'unused-imports/no-unused-imports': 1,
    'unused-imports/no-unused-vars': [
      1,
      {
        vars: 'all',
        varsIgnorePattern: '^_',
        args: 'none',
        // args: 'after-used', // Temporarily turn off
        // argsIgnorePattern: '^_', // Temporarily turn off
      },
    ],
  },
};

/**
 * @config perfectionistConfig
 * Package: 'eslint-plugin-perfectionist'
 * Enforces strict sorting of imports, exports, and objects for better readability.
 */
const customGroups = {
  mui: 'custom-mui',
  auth: 'custom-auth',
  hooks: 'custom-hooks',
  utils: 'custom-utils',
  types: 'custom-types',
  routes: 'custom-routes',
  sections: 'custom-sections',
  components: 'custom-components',
};

const typeGroups = [
  ['type-external', 'type-builtin'],
  { newlinesBetween: 0 },
  ['type-index', 'type-parent', 'type-sibling', 'type-internal'],
];

const perfectionistConfig = {
  plugins: {
    perfectionist: pluginPerfectionist,
  },
  rules: {
    'perfectionist/sort-named-imports': [1, { type: 'line-length', order: 'asc' }],
    'perfectionist/sort-named-exports': [1, { type: 'line-length', order: 'asc' }],
    'perfectionist/sort-exports': [
      1,
      {
        order: 'asc',
        type: 'line-length',
        groups: ['value-export', 'type-export'],
      },
    ],
    'perfectionist/sort-imports': [
      2,
      {
        order: 'asc',
        ignoreCase: true,
        type: 'line-length',
        environment: 'node',
        maxLineLength: undefined,
        newlinesBetween: 1,
        internalPattern: ['^src/.+'],
        groups: [
          'style',
          'side-effect',
          ...typeGroups,
          ['builtin', 'external'],
          customGroups.mui,
          customGroups.routes,
          customGroups.hooks,
          customGroups.utils,
          'internal',
          customGroups.components,
          customGroups.sections,
          customGroups.auth,
          customGroups.types,
          ['parent', 'sibling', 'index'],
          'import',
          'unknown',
        ],
        customGroups: [
          {
            groupName: customGroups.mui,
            elementNamePattern: '^@mui/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.auth,
            elementNamePattern: '^src/auth/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.hooks,
            elementNamePattern: '^src/hooks/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.utils,
            elementNamePattern: '^src/utils/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.types,
            elementNamePattern: '^src/types/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.routes,
            elementNamePattern: '^src/routes/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.sections,
            elementNamePattern: '^src/sections/.+',
            modifiers: ['value'],
          },
          {
            groupName: customGroups.components,
            elementNamePattern: '^src/components/.+',
            modifiers: ['value'],
          },
        ],
      },
    ],
  },
};

// ----------------------------------------------------------------------

const baseConfig = {
  files: ['**/*.{js,jsx,mjs,cjs,ts,tsx}'],
};

const eslintConfig = [
  globalIgnores([
    // Default ignores
    '.next/**',
    'next-env.d.ts',
    'out/**',
    'dist/**',
    'build/**',
    // Auto-generated API client
    'src/lib/swissknife/**',
  ]),
  /********/
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
  },
  /********/
  eslintJs.configs.recommended,
  ...eslintTs.configs.recommended,
  pluginReact.configs.flat.recommended,
  /********/
  baseConfig,
  reactConfig,
  importConfig,
  unusedImportsConfig,
  perfectionistConfig,
];

export default eslintConfig;
