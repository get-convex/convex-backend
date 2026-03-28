import { StorybookConfig } from "@storybook/nextjs-vite";
import path from "path";
import { UserConfig, mergeConfig } from "vite";

const config: StorybookConfig = {
  stories: [
    "../../dashboard/src/**/*.stories.@(js|jsx|ts|tsx)",
    "../../dashboard-common/src/**/*.stories.@(js|jsx|ts|tsx)",
    "../../@convex-dev/design-system/src/**/*.stories.@(js|jsx|ts|tsx)",
  ],
  addons: [
    "@storybook/addon-links",
    "@storybook/addon-themes",
    "@storybook/addon-docs",
    "@storybook/addon-a11y",
    "@storybook/addon-vitest",
  ],
  staticDirs: [
    {
      from: path.resolve(
        import.meta.dirname,
        "../../@convex-dev/design-system/node_modules/@fontsource-variable/inter/files",
      ),
      to: "/assets/files",
    },
  ],
  framework: {
    name: "@storybook/nextjs-vite",
    options: {
      nextConfigPath: path.resolve(
        import.meta.dirname,
        "../../dashboard/next.config.js",
      ),
    },
  },
  viteFinal: async (config) => {
    return mergeConfig(config, {
      css: {
        postcss: path.resolve(
          import.meta.dirname,
          "../../dashboard/postcss.config.js",
        ),
      },
      resolve: {
        alias: {
          // Match dashboard's tsconfig baseUrl: "src" for api/* imports
          api: path.resolve(import.meta.dirname, "../../dashboard/src/api"),
          hooks: path.resolve(import.meta.dirname, "../../dashboard/src/hooks"),
          // Match dashboard/dashboard-common tsconfig path alias "@common/*"
          "@common": path.resolve(
            import.meta.dirname,
            "../../dashboard-common/src",
          ),
          // Match design-system's tsconfig path alias "@ui/*": ["*"]
          "@ui": path.resolve(
            import.meta.dirname,
            "../../@convex-dev/design-system/src",
          ),
          // Storybook's Vite build can't bundle `saffron`'s `.wasm` dependency
          // (see vite:wasm-fallback errors). For Storybook only, swap in a
          // minimal JS mock so stories can render.
          saffron: path.resolve(import.meta.dirname, "./mocks/saffron.ts"),
        },
      },
      server: {
        fs: {
          allow: [
            ...(config.server?.fs?.allow || []),
            path.resolve(import.meta.dirname, "../../"),
          ],
        },
      },
      optimizeDeps: {
        esbuildOptions: {
          tsconfig: "../dashboard/tsconfig.json",
        },
      },
    } satisfies UserConfig);
  },
};

export default config;
