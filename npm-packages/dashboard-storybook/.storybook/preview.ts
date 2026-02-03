import "./preview.css";
import { Preview } from "@storybook/nextjs";
import themeDecorator from "./themeDecorator";
import { RouterContext } from "next/dist/shared/lib/router-context.shared-runtime";
import { sb } from "storybook/test";

// Register modules for mocking in stories
// Note: paths must be relative to this file and include extensions for Node.js resolution
sb.mock(import("../../dashboard/src/api/teams.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/projects.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/profile.ts"), { spy: true });
sb.mock(import("../../dashboard/src/api/deployments.ts"), { spy: true });

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
