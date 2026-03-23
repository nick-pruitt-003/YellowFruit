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
      '.erb/dll',
      '**/*.d.ts',
      '**/*.css.d.ts',
      '**/*.sass.d.ts',
      '**/*.scss.d.ts',
    ],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  // Type-aware React rules with TypeScript project service
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
      // React hooks — rules-of-hooks is in recommended-typescript; add exhaustive-deps explicitly
      '@eslint-react/exhaustive-deps': 'warn',

      // TypeScript overrides
      'no-shadow': 'off',
      '@typescript-eslint/no-shadow': 'error',
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': 'error',

      // Style
      'no-underscore-dangle': 'off',
      'no-restricted-syntax': 'off',
      'no-plusplus': 'off',
      'max-classes-per-file': 'off',
      'no-use-before-define': 'off',
      'no-continue': 'off',
      yoda: 'off',
    },
  },
  // .erb build scripts — relax rules that don't apply to Node build tooling
  {
    files: ['.erb/**/*.{js,ts,mjs,cjs}'],
    rules: {
      'no-console': 'off',
    },
  },
);
