import { configDefaults, defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    setupFiles: ['./src/__tests__/setup.ts'],
    environment: 'node',
    // Claude Code worktrees live under .claude/ and carry their own (stale) copy of the tests
    exclude: [...configDefaults.exclude, '.claude/**'],
  },
});
