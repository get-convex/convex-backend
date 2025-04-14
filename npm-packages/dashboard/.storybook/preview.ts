import "../../dashboard-common/src/styles/globals.css";
// TODO: Figure out why storybook breaks when importing the shared css file
// postcss is not running/saving files for some reason
// import "../../ui/src/styles/shared.css";
import { Preview } from "@storybook/react";
import themeDecorator from "./themeDecorator";
import { RouterContext } from "next/dist/shared/lib/router-context.shared-runtime";

const preview: Preview = {
  parameters: {
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
    nextRouter: {
      Provider: RouterContext.Provider, // next 13 (using next/router) / next < 12
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
