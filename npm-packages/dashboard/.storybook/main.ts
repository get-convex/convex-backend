import { StorybookConfig } from "@storybook/types";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.mdx", "../src/**/*.stories.@(js|jsx|ts|tsx)"],
  addons: [
    "@storybook/addon-links",
    "@storybook/addon-essentials",
    "@storybook/addon-interactions",
    "@storybook/addon-themes",
    {
      name: "@storybook/addon-styling",
      options: {
        postCss: {
          implementation: require("postcss"),
        },
      },
    },
  ],
  framework: {
    name: "@storybook/nextjs",
    options: {},
  },
};

export default config;
