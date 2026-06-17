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
  // Without --turbopack (next build with webpack)
  webpack(config) {
    config.module.rules.push({
      test: /\.svg$/,
      use: ['@svgr/webpack'],
    });

    return config;
  },
  // With Turbopack (Next.js 16 default for dev and build)
  turbopack: {
    rules: {
      '*.svg': {
        loaders: ['@svgr/webpack'],
        as: '*.js',
      },
    },
  },
  // Default to 'standalone' for Node.js server
  // Override with 'export' when BUILD_STATIC_EXPORT=true (bundled with backend)
  output: isStaticExport ? 'export' : 'standalone',
};

export default nextConfig;
