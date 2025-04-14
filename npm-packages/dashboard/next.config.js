const { withSentryConfig } = require("@sentry/nextjs");
const { DefinePlugin } = require("webpack");
const path = require("path");

const ContentSecurityPolicy = `
  frame-ancestors 'self';
`;

const allowedImageDomains = [
  {
    protocol: "https",
    hostname: "avatars.githubusercontent.com",
    pathname: "**",
  },
  {
    protocol: "https",
    hostname: "s.gravatar.com",
    pathname: "**",
  },
  {
    protocol: "https",
    hostname: "**.convex.cloud",
    pathname: "/api/storage/**",
  },
];

const securityHeaders = [
  {
    key: "X-DNS-Prefetch-Control",
    value: "on",
  },
  {
    key: "Strict-Transport-Security",
    value: "max-age=63072000; includeSubDomains; preload",
  },
  {
    key: "X-XSS-Protection",
    value: "1; mode=block",
  },
  {
    key: "X-Frame-Options",
    value: "SAMEORIGIN",
  },
  {
    key: "Referrer-Policy",
    value: "origin-when-cross-origin",
  },
  {
    key: "Content-Security-Policy",
    value: ContentSecurityPolicy.replace(/\s{2,}/g, " ").trim(),
  },
];

/** @type {import('next').NextConfig} */
const nextConfig = {
  swcMinify: true,
  transpilePackages: [],
  reactStrictMode: true,
  async headers() {
    return [
      {
        // Apply these headers to all routes in your application.
        source: "/:path*",
        headers: securityHeaders,
      },
    ];
  },
  async redirects() {
    return [
      {
        source: "/teams/:teamId*",
        destination: "/",
        permanent: false,
      },
      {
        source: "/projects/:projectId*",
        destination: "/",
        permanent: false,
      },
      {
        source: "/t/:team/:project/:deploymentName/cron-jobs",
        destination: "/t/:team/:project/:deploymentName/schedules/functions",
        permanent: false,
      },
      {
        source: "/t/:team/:project/:deploymentName/schedules",
        destination: "/t/:team/:project/:deploymentName/schedules/functions",
        permanent: false,
      },
      {
        source: "/t/:team/:project/:deploymentName/settings/snapshot-export",
        destination: "/t/:team/:project/:deploymentName/settings/snapshots",
        permanent: false,
      },
    ];
  },
  sentry: {
    // The Webpack plugin attempts to upload sourcemaps on every production build, which requires having a Sentry auth
    // token. With Rush, all builds are production builds so this is no good. We only want this to happen on a real
    // deployment.
    disableServerWebpackPlugin: !process.env.NETLIFY && !process.env.VERCEL,
    disableClientWebpackPlugin: !process.env.NETLIFY && !process.env.VERCEL,
    hideSourceMaps: true,
  },
  images: {
    domains:
      process.env.VERCEL_ENV === "production"
        ? undefined
        : ["127.0.0.1", "s.gravatar.com", "avatars.githubusercontent.com"],
    remotePatterns: allowedImageDomains,
  },
  experimental: {
    webpackBuildWorker: true,
  },
  // from https://github.com/vercel/next.js/blob/c110dfd57c754f88cb239dc154a4b7d49e5696a3/examples/with-webassembly/next.config.js
  webpack(config, { isServer, dev }) {
    config.resolve.symlinks = true; // Ensure Webpack follows symlinks
    // Force Webpack to watch changes in the src directory of local packages
    config.watchOptions = {
      ignored: [
        "**/node_modules/**", // Ignore other node_modules
        "!**/node_modules/dashboard-common/src/**", // But watch src/
      ],
    };

    // next.config.js
    config.module.rules.push({
      test: /\.(mp3|wav|m4a)$/,
      use: {
        loader: "file-loader",
      },
    });
    // Use the client static directory in the server bundle and prod mode
    // Fixes `Error occurred prerendering page "/"`
    config.output.webassemblyModuleFilename =
      isServer && !dev
        ? "../static/wasm/[modulehash].wasm"
        : "static/wasm/[modulehash].wasm";

    config.module.rules.push({
      test: /\.svg$/,
      use: ["@svgr/webpack"],
    });

    // Since Webpack 5 doesn't enable WebAssembly by default, we should do it manually
    config.experiments = { ...config.experiments, asyncWebAssembly: true };
    // Fix warnings for async functions in the browser (https://github.com/vercel/next.js/issues/64792)
    // We only use this on the client so we don't care about the server environment
    // not supporting it.
    if (!isServer) {
      config.output.environment = {
        ...config.output.environment,
        asyncFunction: true,
      };
    }

    config.resolve.extensions.push(".js", ".jsx", ".ts", ".tsx");

    config.plugins.push(
      new DefinePlugin({
        __SENTRY_DEBUG__: false,
        // Consider turning off __SENTRY_TRACING__
        // if we don't care much about web vitals
        // __SENTRY_TRACING__: false,
      }),
    );

    return config;
  },
  eslint: {
    // eslint is run in a separate step during CI, so don't fail the build on lint errors
    ignoreDuringBuilds: true,
  },
  env: {
    NEXT_PUBLIC_VERCEL_GIT_COMMIT_SHA: process.env.VERCEL_GIT_COMMIT_SHA || "",
  },
};

const withBundleAnalyzer = require("@next/bundle-analyzer")({
  enabled: process.env.ANALYZE === "true",
});

module.exports = withBundleAnalyzer(
  withSentryConfig(nextConfig, {
    dryRun: process.env.VERCEL && process.env.VERCEL_ENV !== "production",
    release: process.env.VERCEL_GIT_COMMIT_SHA,
    silent: true,
  }),
  {
    widenClientFileUpload: true,
  },
);
