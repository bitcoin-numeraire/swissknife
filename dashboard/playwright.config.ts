import { devices, defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:8080',
    trace: 'on-first-retry',
  },
  webServer: {
    command:
      'BUILD_STATIC_EXPORT=false NEXT_PUBLIC_AUTH_METHOD=mock-oauth2 NEXT_PUBLIC_SERVER_URL=http://127.0.0.1:1993 NEXT_PUBLIC_MOCK_OAUTH2_TOKEN_URL=http://127.0.0.1:8090/default/token NEXT_PUBLIC_MOCK_OAUTH2_CLIENT_SECRET=dev-secret yarn dev',
    reuseExistingServer: process.env.PLAYWRIGHT_REUSE_SERVER === 'true',
    timeout: 120_000,
    url: 'http://localhost:8080/login',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
