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
