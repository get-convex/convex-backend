import "../src/styles/globals.css";
import { Preview } from "@storybook/react";
import themeDecorator from "./themeDecorator";

const preview: Preview = {
  parameters: {
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
  },

  decorators: [
    themeDecorator({
      themes: {
        light: "light",
        dark: "dark",
      },
      defaultTheme: "light",
    }),
  ],
};

export default preview;
