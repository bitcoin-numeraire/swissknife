// eslint-disable-next-line import/no-extraneous-dependencies
import withPWA from 'next-pwa';

// Use environment variable to determine build mode
// - 'true' = static export (for bundling with backend)
// - undefined/false = standalone (for standalone dashboard deployment)
const isStaticExport = process.env.BUILD_STATIC_EXPORT === 'true';

/**
 * @type {import('next').NextConfig}
 */
const nextConfig = {
  trailingSlash: true,
  basePath: process.env.NEXT_PUBLIC_BASE_PATH,
  env: {
    BUILD_STATIC_EXPORT: isStaticExport ? 'true' : 'false',
  },
  modularizeImports: {
    '@mui/icons-material': {
      transform: '@mui/icons-material/{{member}}',
    },
    '@mui/material': {
      transform: '@mui/material/{{member}}',
    },
    '@mui/lab': {
      transform: '@mui/lab/{{member}}',
    },
  },
  webpack(config) {
    config.module.rules.push({
      test: /\.svg$/,
      use: ['@svgr/webpack'],
    });

    return config;
  },
  // Default to 'standalone' for Node.js server
  // Override with 'export' when BUILD_STATIC_EXPORT=true (bundled with backend)
  output: isStaticExport ? 'export' : 'standalone',
};

export default withPWA({
  dest: 'public',
  disable: process.env.NODE_ENV === 'development',
})(nextConfig);
