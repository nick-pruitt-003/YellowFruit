import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import eslintReact from '@eslint-react/eslint-plugin';
import globals from 'globals';

export default tseslint.config(
  {
    ignores: [
      'node_modules',
      'release/app/dist',
      'release/build',
      '.erb/**',        // webpack build scripts — not production code
      '.eslintrc.js',   // legacy config file, ignored by flat config anyway
      '**/*.d.ts',
      '**/*.css.d.ts',
      '**/*.sass.d.ts',
      '**/*.scss.d.ts',
    ],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  eslintReact.configs['recommended-typescript'],
  {
    files: ['**/*.ts', '**/*.tsx'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      parser: tseslint.parser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: 'module',
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      // ── TypeScript ─────────────────────────────────────────────────────────
      'no-shadow': 'off',
      '@typescript-eslint/no-shadow': 'error',
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': ['error', {
        vars: 'all',
        args: 'after-used',
        argsIgnorePattern: '^_',   // honour _param convention for unused args
        ignoreRestSiblings: true,
      }],
      // Legacy parsing/IPC code uses `any` extensively; warn but don't block.
      '@typescript-eslint/no-explicit-any': 'warn',
      // Electron main process uses require() for dynamic imports.
      '@typescript-eslint/no-require-imports': 'off',

      // ── Actual bug catchers ─────────────────────────────────────────────
      'no-useless-assignment': 'error',
      '@typescript-eslint/no-wrapper-object-types': 'error',
      '@typescript-eslint/no-empty-object-type': 'error',

      // ── React 19 migration hints — informational, not bugs ──────────────
      '@eslint-react/no-use-context': 'off',
      '@eslint-react/no-context-provider': 'off',
      '@eslint-react/no-forward-ref': 'off',
      '@eslint-react/use-state': 'off',
      '@eslint-react/no-unnecessary-use-prefix': 'off',
      '@eslint-react/no-array-index-key': 'warn',
      '@eslint-react/set-state-in-effect': 'warn',
      '@eslint-react/exhaustive-deps': 'warn',
      '@eslint-react/purity': 'warn',

      // ── Style ──────────────────────────────────────────────────────────
      'no-underscore-dangle': 'off',
      'no-restricted-syntax': 'off',
      'no-plusplus': 'off',
      'max-classes-per-file': 'off',
      'no-use-before-define': 'off',
      'no-continue': 'off',
      yoda: 'off',
    },
  },
);
