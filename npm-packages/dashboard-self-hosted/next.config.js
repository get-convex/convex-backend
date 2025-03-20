const path = require("path");

const ContentSecurityPolicy = `
  frame-ancestors 'self';
`;

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

const optionsForExport = {
  output: "export",
  images: {
    unoptimized: true,
  },
};
const optionsForBuild = {
  output: "standalone",
  async headers() {
    return [
      {
        // Apply these headers to all routes in your application.
        source: "/:path*",
        headers: securityHeaders,
      },
    ];
  },
};

/** @type {import('next').NextConfig} */
const nextConfig = {
  swcMinify: true,
  transpilePackages: [],
  reactStrictMode: true,
  ...(process.env.BUILD_TYPE === "export" ? optionsForExport : optionsForBuild),
  experimental: {
    webpackBuildWorker: true,
  },
  webpack(config, { isServer, dev }) {
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

    config.resolve.alias = {
      ...config.resolve.alias,
      "dashboard-common": path.resolve(__dirname, "../dashboard-common/src"),
      "@local/elements": path.resolve(
        __dirname,
        "../dashboard-common/src/elements",
      ),
      "@local/lib": path.resolve(__dirname, "../dashboard-common/src/lib"),
      "@local/features": path.resolve(
        __dirname,
        "../dashboard-common/src/features",
      ),
      "@local/layouts": path.resolve(
        __dirname,
        "../dashboard-common/src/layouts",
      ),
    };
    config.resolve.symlinks = true; // Ensure Webpack follows symlinks
    // Force Webpack to watch changes in the src directory of local packages
    config.watchOptions = {
      ignored: [
        "**/node_modules/**", // Ignore other node_modules
        "!**/node_modules/dashboard-common/src/**", // But watch src/
      ],
    };
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

    return config;
  },
};

module.exports = nextConfig;