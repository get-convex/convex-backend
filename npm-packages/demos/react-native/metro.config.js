const { getDefaultConfig } = require("expo/metro-config");
const path = require("path");

const projectRoot = __dirname;
const monorepoRoot = path.resolve(projectRoot, "../..");

const config = getDefaultConfig(projectRoot);

config.watchFolders = [monorepoRoot];

config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, "node_modules"),
  path.resolve(monorepoRoot, "common/temp/node_modules"),
];

config.resolver.unstable_enableSymlinks = true;
config.resolver.unstable_enablePackageExports = true;

// In this Rush + pnpm monorepo, the `convex` workspace package brings its own
// React 18 copy at `npm-packages/convex/node_modules/react`, while this demo
// uses React 19. Without intervention Metro bundles both copies and React
// throws "different React version" at runtime. Force a single React copy via
// three layers: blockList (forbid bad paths), extraNodeModules (redirect
// bare-specifier lookups), and resolveRequest (intercept everything else).

config.resolver.blockList = [
  /npm-packages\/convex\/node_modules\/react\/.*/,
  /npm-packages\/convex\/node_modules\/react-dom\/.*/,
  /npm-packages\/convex\/node_modules\/scheduler\/.*/,
  /common\/temp\/node_modules\/\.pnpm\/react@(?!19)[^/]+\/.*/,
  /common\/temp\/node_modules\/\.pnpm\/react-dom@(?!19)[^/]+\/.*/,
];

config.resolver.extraNodeModules = {
  react: path.resolve(projectRoot, "node_modules/react"),
  "react-dom": path.resolve(projectRoot, "node_modules/react-dom"),
};

const PINNED = /^(react|react-dom|scheduler)(\/.*)?$/;
config.resolver.resolveRequest = (context, moduleName, platform) => {
  if (PINNED.test(moduleName)) {
    return {
      filePath: require.resolve(moduleName, { paths: [projectRoot] }),
      type: "sourceFile",
    };
  }
  return context.resolveRequest(context, moduleName, platform);
};

module.exports = config;
