import type { MetadataRoute } from 'next';

import { CONFIG } from 'src/global-config';

export const dynamic = 'force-static';

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: CONFIG.appName,
    short_name: CONFIG.appName,
    start_url: '/',
    icons: [
      {
        src: `${CONFIG.assetsDir}/favicon/android-chrome-192x192.png`,
        sizes: '192x192',
        type: 'image/png',
      },
      {
        src: `${CONFIG.assetsDir}/favicon/android-chrome-512x512.png`,
        sizes: '512x512',
        type: 'image/png',
      },
    ],
    theme_color: '#ffffff',
    background_color: '#ffffff',
    display: 'standalone',
  };
}
