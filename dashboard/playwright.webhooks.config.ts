import { devices, defineConfig } from '@playwright/test';

// Uses the integration stack's real OAuth2 provider and a running SwissKnife
// backend. See e2e/README.md. Successful operations use the real API;
// dedicated error cases inject failed responses.
export default defineConfig({
  testDir: './e2e',
  testMatch: 'webhooks.spec.ts',
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  expect: { timeout: 15_000 },
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:8180',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'yarn next dev --webpack -p 8180',
    env: {
      BUILD_STATIC_EXPORT: 'false',
      NEXT_PUBLIC_AUTH_METHOD: 'mock-oauth2',
      NEXT_PUBLIC_SERVER_URL: process.env.SWISSKNIFE_E2E_API ?? 'http://127.0.0.1:21993',
      NEXT_PUBLIC_MOCK_OAUTH2_TOKEN_URL: `${process.env.SWISSKNIFE_E2E_OAUTH2 ?? 'http://127.0.0.1:8090'}/default/token`,
    },
    reuseExistingServer: process.env.PLAYWRIGHT_REUSE_SERVER === 'true',
    url: 'http://localhost:8180/login',
    timeout: 120_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
