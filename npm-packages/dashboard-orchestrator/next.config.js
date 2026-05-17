const ContentSecurityPolicy = `
  frame-ancestors 'self';
`;

const securityHeaders = [
  { key: "X-DNS-Prefetch-Control", value: "on" },
  {
    key: "Strict-Transport-Security",
    value: "max-age=63072000; includeSubDomains; preload",
  },
  { key: "X-XSS-Protection", value: "1; mode=block" },
  { key: "Referrer-Policy", value: "origin-when-cross-origin" },
];

const optionsForExport = {
  output: "export",
  images: { unoptimized: true },
};

const optionsForBuild = {
  output: "standalone",
  async headers() {
    return [
      {
        source: "/:path*",
        headers: [
          ...securityHeaders,
          { key: "X-Frame-Options", value: "SAMEORIGIN" },
          {
            key: "Content-Security-Policy",
            value: ContentSecurityPolicy.replace(/\s{2,}/g, " ").trim(),
          },
        ],
      },
    ];
  },
};

/** @type {import('next').NextConfig} */
const nextConfig = {
  transpilePackages: [],
  reactStrictMode: true,
  ...(process.env.BUILD_TYPE === "export" ? optionsForExport : optionsForBuild),
  experimental: {
    webpackBuildWorker: true,
  },
  webpack(config, { isServer, dev }) {
    // Match dashboard-self-hosted's webasm output path so .wasm modules
    // resolve correctly in both server and client bundles.
    config.output.webassemblyModuleFilename =
      isServer && !dev
        ? "../static/wasm/[modulehash].wasm"
        : "static/wasm/[modulehash].wasm";

    config.module.rules.push({
      test: /\.svg$/,
      use: ["@svgr/webpack"],
    });
    config.resolve.symlinks = true;
    config.watchOptions = {
      ignored: [
        "**/node_modules/**",
        "!**/node_modules/dashboard-common/src/**",
      ],
    };
    // dashboard-common pulls in `@cloudflare/saffron`, which ships a .wasm
    // module. Webpack 5 disables WebAssembly by default; we enable the
    // async variant here (same as dashboard-self-hosted's next.config.js).
    config.experiments = { ...config.experiments, asyncWebAssembly: true };
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
