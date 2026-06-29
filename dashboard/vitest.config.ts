import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

export default defineConfig({
  resolve: {
    alias: {
      src: fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    css: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
  },
});
