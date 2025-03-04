import { StorybookConfig } from "@storybook/types";
import path from "path";

const config: StorybookConfig = {
  stories: [
    "../src/**/*.stories.mdx",
    "../src/**/*.stories.@(js|jsx|ts|tsx)",
    "../../dashboard-common/src/**/*.stories.mdx",
    "../../dashboard-common/src/**/*.stories.@(js|jsx|ts|tsx)",
  ],
  addons: [
    "@storybook/addon-links",
    "@storybook/addon-essentials",
    "@storybook/addon-interactions",
    "@storybook/addon-themes",
    {
      name: "@storybook/addon-styling",
      options: {
        postCss: true,
      },
    },
  ],
  framework: {
    name: "@storybook/nextjs",
    options: {
      webpackFinal: async (config) => {
        // Configure aliases
        config.resolve = {
          ...config.resolve,
          alias: {
            ...config.resolve?.alias,
            "dashboard-common": path.resolve(
              __dirname,
              "../../dashboard-common/src",
            ),
            "@common": path.resolve(__dirname, "../../dashboard-common/src"),
          },
        };

        return config;
      },
    },
  },
};

export default config;
