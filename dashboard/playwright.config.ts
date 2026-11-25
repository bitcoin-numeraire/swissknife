import { devices, defineConfig } from '@playwright/test';

const regtestDashboard = process.env.SWISSKNIFE_POS_REGTEST_DASHBOARD;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  globalSetup: regtestDashboard ? undefined : './e2e/global-setup.ts',
  globalTeardown: regtestDashboard ? undefined : './e2e/global-teardown.ts',
  reporter: [['list']],
  use: {
    baseURL: regtestDashboard || 'http://localhost:8080',
    channel: process.env.PLAYWRIGHT_CHANNEL || undefined,
    trace: 'on-first-retry',
  },
  webServer: regtestDashboard
    ? undefined
    : {
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
