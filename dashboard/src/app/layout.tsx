import 'src/global.css';

// ----------------------------------------------------------------------

import type { Viewport } from 'next';

import { CONFIG } from 'src/config-global';
import { primary } from 'src/theme/core/palette';
import { LocalizationProvider } from 'src/locales';
import { detectLanguage } from 'src/locales/server';
import { I18nProvider } from 'src/locales/i18n-provider';
import { ThemeProvider } from 'src/theme/theme-provider';
import { getInitColorSchemeScript } from 'src/theme/color-scheme-script';

import { Snackbar } from 'src/components/snackbar';
import { ProgressBar } from 'src/components/progress-bar';
import { MotionLazy } from 'src/components/animate/motion-lazy';
import { detectSettings } from 'src/components/settings/server';
import { SettingsDrawer, defaultSettings, SettingsProvider } from 'src/components/settings';

import { AuthProvider as JwtAuthProvider } from 'src/auth/context/jwt';
import { AuthProvider as Auth0AuthProvider } from 'src/auth/context/auth0';
import { AuthProvider as SupabaseAuthProvider } from 'src/auth/context/supabase';

// ----------------------------------------------------------------------

const AuthProvider =
  (CONFIG.auth.method === 'supabase' && SupabaseAuthProvider) || (CONFIG.auth.method === 'auth0' && Auth0AuthProvider) || JwtAuthProvider;

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  themeColor: primary.main,
};

export const metadata = {
  title: CONFIG.site.name,
  description: `${CONFIG.site.name}, your assistant to handle everything Bitcoin`,
  keywords: 'bitcoin,numeraire,swissknife,blockchain,lightning,rgb,protocol,smartcontract,decentralised,network,taproot-assets',
  manifest: '/site.webmanifest',
  icons: [
    { rel: 'icon', url: `${CONFIG.site.basePath}/favicon/favicon.ico` },
    {
      rel: 'icon',
      type: 'image/png',
      sizes: '16x16',
      url: `${CONFIG.site.basePath}/favicon/favicon-16x16.png`,
    },
    {
      rel: 'icon',
      type: 'image/png',
      sizes: '32x32',
      url: `${CONFIG.site.basePath}/favicon/favicon-32x32.png`,
    },
    {
      rel: 'apple-touch-icon',
      sizes: '180x180',
      url: `${CONFIG.site.basePath}/favicon/apple-touch-icon.png`,
    },
    {
      rel: 'mask-icon',
      color: '#5bbad5',
      url: `${CONFIG.site.basePath}/favicon/safari-pinned-tab.svg`,
    },
  ],
};

type Props = {
  children: React.ReactNode;
};

export default async function RootLayout({ children }: Props) {
  const lang = CONFIG.isStaticExport ? 'en' : await detectLanguage();

  const settings = CONFIG.isStaticExport ? defaultSettings : await detectSettings();

  return (
    <html lang={lang ?? 'en'} suppressHydrationWarning>
      <body>
        {getInitColorSchemeScript}

        <I18nProvider lang={CONFIG.isStaticExport ? undefined : lang}>
          <LocalizationProvider>
            <AuthProvider>
              <SettingsProvider settings={settings} caches={CONFIG.isStaticExport ? 'localStorage' : 'cookie'}>
                <ThemeProvider>
                  <MotionLazy>
                    <Snackbar />
                    <ProgressBar />
                    <SettingsDrawer hideDirection hideFont hidePresets hideNavLayout hideNavColor />
                    {children}
                  </MotionLazy>
                </ThemeProvider>
              </SettingsProvider>
            </AuthProvider>
          </LocalizationProvider>
        </I18nProvider>
      </body>
    </html>
  );
}
